#![cfg_attr(windows, windows_subsystem = "windows")]

//! NETS - 네트워크 모니터
//! * 주변 네트워크 장치(IP / 이름 / 제조사 / MAC / 응답시간) 탐색
//! * 실시간 송수신 속도 추산(Mbps, 1초 단위)
//! * 외부 다운로드 속도를 5초마다 100KB 기준으로 측정하고 그래프 표시
//! * 내 네트워크 설정(IP, DNS, 게이트웨이, MAC 등) 표시
//! * 다국어 지원 (한국어, English, Français, Deutsch, 日本語, 中文, Русский)

mod conns;
mod i18n;
mod netinfo;
mod oui;
mod scanner;
mod settings;
mod shares;
mod speed;
mod traffic;

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::net::Ipv4Addr;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, RichText};
use egui_plot::{Line, Plot, PlotPoints};

use i18n::{t, tf, Lang};
use scanner::Device;
use speed::{RegionPoint, RegionResult, RegionStatus, Sample};
use traffic::{Bucket5, TrafficSample};

const GREEN: Color32 = Color32::from_rgb(64, 200, 110);
const YELLOW: Color32 = Color32::from_rgb(240, 190, 60);
const RED: Color32 = Color32::from_rgb(235, 90, 90);
const MUTED: Color32 = Color32::from_rgb(140, 145, 160);
const ACCENT: Color32 = Color32::from_rgb(90, 160, 255);
const UPLOAD: Color32 = Color32::from_rgb(240, 150, 90);

/// 백그라운드 스레드와 UI가 공유하는 상태
#[derive(Clone)]
pub struct Shared {
    pub devices: Vec<Device>,
    pub local_ip: Option<Ipv4Addr>,
    pub scanning: bool,
    pub last_scan: Option<Instant>,
    pub samples: VecDeque<Sample>,
    pub testing: bool,
    pub next_test: Option<Instant>,
    pub last_error: Option<String>,
    pub started: Instant,
    pub net_info: Option<netinfo::NetInfo>,
    pub traffic: VecDeque<TrafficSample>,
    pub regions: Vec<RegionResult>,
    pub region_history: VecDeque<RegionPoint>,
    pub share_info: shares::ShareInfo,
    pub traffic5: VecDeque<Bucket5>,
    pub conn_info: conns::ConnInfo,
}

struct NetsApp {
    shared: Arc<Mutex<Shared>>,
    rescan_tx: mpsc::Sender<()>,
    /// 시작 시 불러온 배치 (패널 기본 크기로 사용)
    initial: settings::Layout,
    /// 마지막으로 저장한 배치 (바뀔 때만 파일 기록)
    saved: settings::Layout,
    show_help: bool,
    /// 창 제목에 반영한 언어
    title_lang: Option<Lang>,
}

impl NetsApp {
    fn new(cc: &eframe::CreationContext<'_>, initial: settings::Layout) -> Self {
        setup_fonts(&cc.egui_ctx);
        // 시스템이 라이트 모드여도 항상 다크 테마 유지
        cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let shared = Arc::new(Mutex::new(Shared {
            devices: Vec::new(),
            local_ip: None,
            scanning: false,
            last_scan: None,
            samples: VecDeque::new(),
            testing: false,
            next_test: None,
            last_error: None,
            started: Instant::now(),
            net_info: None,
            traffic: VecDeque::new(),
            regions: Vec::new(),
            region_history: VecDeque::new(),
            share_info: Default::default(),
            traffic5: VecDeque::new(),
            conn_info: Default::default(),
        }));

        let (tx, rx) = mpsc::channel();
        {
            let s = Arc::clone(&shared);
            thread::spawn(move || scanner::run(s, rx));
        }
        {
            let s = Arc::clone(&shared);
            thread::spawn(move || speed::run(s));
        }
        {
            let s = Arc::clone(&shared);
            thread::spawn(move || netinfo::run(s));
        }
        {
            let s = Arc::clone(&shared);
            thread::spawn(move || traffic::run(s));
        }
        {
            let s = Arc::clone(&shared);
            thread::spawn(move || shares::run(s));
        }
        {
            let s = Arc::clone(&shared);
            thread::spawn(move || conns::run(s));
        }

        Self {
            shared,
            rescan_tx: tx,
            initial,
            saved: initial,
            show_help: false,
            title_lang: None,
        }
    }
}

/// 여러 언어를 보이도록 글꼴을 순서대로 등록합니다 (앞의 글꼴 우선, 없는 글자는 다음 글꼴에서 찾음).
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let candidates = [
        ("arial", "C:\\Windows\\Fonts\\arial.ttf"),
        ("malgun", "C:\\Windows\\Fonts\\malgun.ttf"),
        ("msyh", "C:\\Windows\\Fonts\\msyh.ttc"),
        ("msgothic", "C:\\Windows\\Fonts\\msgothic.ttc"),
        ("nanum", "/usr/share/fonts/truetype/nanum/NanumGothic.ttf"),
    ];
    for (name, path) in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                name.to_owned(),
                egui::FontData::from_owned(bytes),
            );
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .push(name.to_owned());
            }
        }
    }
    ctx.set_fonts(fonts);
}

fn card(ui: &mut egui::Ui, title: String, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.label(RichText::new(title).strong().size(17.0).color(ACCENT));
            ui.add_space(6.0);
            add(ui);
        });
}

fn kv(ui: &mut egui::Ui, key: String, value: String) {
    ui.label(RichText::new(format!("{key}: ")).color(MUTED));
    ui.label(RichText::new(value).strong());
    ui.add_space(10.0);
}

/// 표 머리글 (굵게)
fn hdr(ui: &mut egui::Ui, text: String) {
    ui.label(RichText::new(text).strong());
}

/// 바이트를 KB/MB/GB 단위 문자열로
fn fmt_bytes(b: u64) -> String {
    let f = b as f64;
    if f >= 1e9 {
        format!("{:.2} GB", f / 1e9)
    } else if f >= 1e6 {
        format!("{:.2} MB", f / 1e6)
    } else {
        format!("{:.1} KB", f / 1e3)
    }
}

fn fmt_time(t: f64) -> String {
    let total = t as u64;
    format!("{:02}:{:02}:{:02}", total / 3600, (total % 3600) / 60, total % 60)
}

fn speed_color(mbps: Option<f64>) -> Color32 {
    match mbps {
        None => MUTED,
        Some(m) if m >= 20.0 => GREEN,
        Some(m) if m >= 5.0 => YELLOW,
        Some(_) => RED,
    }
}

fn devices_card(ui: &mut egui::Ui, snap: &Shared) {
    card(ui, t("card_devices"), |ui| {
        let online = snap.devices.iter().filter(|d| d.by_ping).count();
        ui.label(
            RichText::new(tf(
                "devices_summary",
                &[
                    ("n", snap.devices.len().to_string()),
                    ("m", online.to_string()),
                ],
            ))
            .color(MUTED),
        );
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for d in &snap.devices {
                    let is_me = is_my_ip(snap, d.ip);
                    let is_gw = d.ip.octets()[3] == 1;
                    let (kind, color) = if is_me {
                        (t("me"), GREEN)
                    } else if is_gw {
                        (t("gateway_kind"), YELLOW)
                    } else {
                        (t("device"), Color32::LIGHT_GRAY)
                    };
                    let name = match &d.hostname {
                        Some(h) => h.clone(),
                        None if is_me => t("me"),
                        None => t("no_name"),
                    };
                    let vendor = match d.vendor {
                        Some(v) => v.to_string(),
                        None => t("vendor_unknown"),
                    };
                    let mac = d.mac.clone().unwrap_or_else(|| "-".to_string());
                    let (status, status_color) = if d.by_ping {
                        (t("online"), GREEN)
                    } else {
                        (t("arp_only"), MUTED)
                    };
                    let lat = match d.latency_ms {
                        Some(ms) if d.by_ping && !is_me => format!("{ms} ms"),
                        _ => "-".to_string(),
                    };

                    egui::Frame::group(ui.style())
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(kind).color(color).strong());
                                ui.label(RichText::new(d.ip.to_string()).monospace());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.label(RichText::new(status).color(status_color));
                                        ui.label(RichText::new(lat).color(MUTED));
                                    },
                                );
                            });
                            ui.label(tf("name_label", &[("name", name)]));
                            ui.label(
                                RichText::new(tf("vendor_label", &[("vendor", vendor)]))
                                    .color(MUTED),
                            );
                            ui.label(
                                RichText::new(tf("mac_label", &[("mac", mac)]))
                                    .monospace()
                                    .color(MUTED),
                            );
                        });
                    ui.add_space(4.0);
                }
            });
    });
}

/// 실시간 송수신 속도 추산 블록 (1초 단위, Mbps)
fn traffic_block(ui: &mut egui::Ui, snap: &Shared) {
    let latest = snap.traffic.back();
    let rx_text = latest
        .map(|t| format!("{:.2}", t.rx_mbps))
        .unwrap_or_else(|| "--".to_string());
    let tx_text = latest
        .map(|t| format!("{:.2}", t.tx_mbps))
        .unwrap_or_else(|| "--".to_string());

    ui.label(RichText::new(t("live_title")).color(MUTED));
    ui.horizontal(|ui| {
        ui.label(RichText::new(t("rx_label")).color(ACCENT));
        ui.label(RichText::new(rx_text).size(30.0).strong().color(ACCENT));
        ui.label(RichText::new("Mbps").color(MUTED));
        ui.add_space(16.0);
        ui.label(RichText::new(t("tx_label")).color(UPLOAD));
        ui.label(RichText::new(tx_text).size(30.0).strong().color(UPLOAD));
        ui.label(RichText::new("Mbps").color(MUTED));
    });

    let rx_pts: Vec<[f64; 2]> = snap.traffic.iter().map(|t| [t.t, t.rx_mbps]).collect();
    let tx_pts: Vec<[f64; 2]> = snap.traffic.iter().map(|t| [t.t, t.tx_mbps]).collect();
    Plot::new("traffic_plot")
        .height(150.0)
        .allow_scroll(false)
        .allow_zoom(false)
        .allow_drag(false)
        .x_axis_label(t("elapsed"))
        .y_axis_label("Mbps")
        .legend(egui_plot::Legend::default())
        .show(ui, |plot_ui| {
            plot_ui.line(
                Line::new(PlotPoints::new(rx_pts))
                    .color(ACCENT)
                    .name(t("rx_legend")),
            );
            plot_ui.line(
                Line::new(PlotPoints::new(tx_pts))
                    .color(UPLOAD)
                    .name(t("tx_legend")),
            );
        });
}

/// 지역별 속도 그래프 (지역마다 선 하나)
fn region_plot(ui: &mut egui::Ui, snap: &Shared) {
    const LINE_COLORS: [Color32; 6] = [
        ACCENT,
        GREEN,
        YELLOW,
        UPLOAD,
        Color32::from_rgb(190, 120, 255),
        RED,
    ];
    ui.label(RichText::new(t("region_graph_title")).strong());
    Plot::new("region_plot")
        .height(220.0)
        .allow_scroll(false)
        .allow_zoom(false)
        .allow_drag(false)
        .x_axis_label(t("elapsed"))
        .y_axis_label("KB/s")
        .legend(egui_plot::Legend::default())
        // 마우스를 올리면 X(시간) 값 대신 각국의 해당 시점 속도를 표시
        .label_formatter({
            let hist: Vec<RegionPoint> = snap.region_history.iter().cloned().collect();
            move |_name, pt| {
                let nearest = hist.iter().min_by(|a, b| {
                    (a.t - pt.x)
                        .abs()
                        .partial_cmp(&(b.t - pt.x).abs())
                        .unwrap_or(Ordering::Equal)
                });
                match nearest {
                    Some(p) => p
                        .values
                        .iter()
                        .map(|(n, v)| match v {
                            Some(kb) => format!("{}: {kb:.1} KB/s", t(n)),
                            None => format!("{}: -", t(n)),
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                    None => String::new(),
                }
            }
        })
        .show(ui, |plot_ui| {
            let names: Vec<&'static str> = snap
                .region_history
                .back()
                .map(|p| p.values.iter().map(|(n, _)| *n).collect())
                .unwrap_or_default();
            for (i, name) in names.iter().enumerate() {
                let pts: Vec<[f64; 2]> = snap
                    .region_history
                    .iter()
                    .filter_map(|p| p.values.get(i).and_then(|(_, v)| *v).map(|kb| [p.t, kb]))
                    .collect();
                plot_ui.line(
                    Line::new(PlotPoints::new(pts))
                        .color(LINE_COLORS[i % LINE_COLORS.len()])
                        .name(t(name)),
                );
            }
        });
}

/// 지역별 다운로드 속도 표 (100KB 기준, 최근 주기)
fn region_table(ui: &mut egui::Ui, snap: &Shared) {
    ui.label(RichText::new(t("region_table_title")).strong());
    egui::Grid::new("regions")
        .striped(true)
        .num_columns(5)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            hdr(ui, t("region"));
            hdr(ui, "KB/s".to_string());
            hdr(ui, "Mbps".to_string());
            hdr(ui, t("hdr_resp"));
            hdr(ui, t("status"));
            ui.end_row();

            // 기본 측정 (Cloudflare)
            let base = snap.samples.back();
            ui.label(t(speed::DEFAULT_NAME));
            ui.label(base.map(|s| format!("{:.1}", s.kb_s)).unwrap_or_else(|| "-".into()));
            ui.label(base.map(|s| format!("{:.2}", s.mbps)).unwrap_or_else(|| "-".into()));
            ui.label(base.map(|s| format!("{:.0} ms", s.latency_ms)).unwrap_or_else(|| "-".into()));
            ui.label(if base.is_some() {
                tf("status_done", &[("n", "100".to_string())])
            } else {
                t("waiting")
            });
            ui.end_row();

            // 빠른 순서로 정렬 (측정 실패/미설정은 맨 아래)
            let mut rows: Vec<&RegionResult> = snap.regions.iter().collect();
            rows.sort_by(|a, b| {
                b.mbps
                    .unwrap_or(-1.0)
                    .partial_cmp(&a.mbps.unwrap_or(-1.0))
                    .unwrap_or(Ordering::Equal)
            });

            for r in rows {
                let ok = r.mbps.is_some();
                let color = if ok { Color32::LIGHT_GRAY } else { MUTED };
                ui.label(RichText::new(t(r.name)).color(color));
                ui.label(
                    RichText::new(r.kb_s.map(|v| format!("{v:.1}")).unwrap_or_else(|| "-".into()))
                        .color(color),
                );
                ui.label(
                    RichText::new(r.mbps.map(|v| format!("{v:.2}")).unwrap_or_else(|| "-".into()))
                        .color(speed_color(r.mbps))
                        .strong(),
                );
                ui.label(
                    RichText::new(
                        r.latency_ms
                            .map(|v| format!("{v:.0} ms"))
                            .unwrap_or_else(|| "-".into()),
                    )
                    .color(color),
                );
                let status = match &r.status {
                    RegionStatus::Done(kb) => {
                        tf("status_done", &[("n", kb.to_string())])
                    }
                    RegionStatus::Unset => t("status_unset"),
                    RegionStatus::Failed(e) => e.clone(),
                };
                ui.label(RichText::new(status).color(if ok { GREEN } else { MUTED }));
                ui.end_row();
            }
        });
}

fn speed_card(ui: &mut egui::Ui, snap: &Shared) {
    card(ui, t("card_speed"), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                traffic_block(ui, snap);

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                ui.label(RichText::new(t("dl_title")).strong());
                let latest = snap.samples.back();
                let kb_text = latest
                    .map(|s| format!("{:.1}", s.kb_s))
                    .unwrap_or_else(|| "--".into());
                let mbps_text = latest
                    .map(|s| format!("{:.2}", s.mbps))
                    .unwrap_or_else(|| "--".into());
                let color = speed_color(latest.map(|s| s.mbps));

                ui.horizontal(|ui| {
                    ui.label(RichText::new(kb_text).size(34.0).strong().color(color));
                    ui.label(RichText::new("KB/s").size(16.0).color(MUTED));
                    ui.add_space(14.0);
                    ui.label(RichText::new(mbps_text).size(24.0).strong().color(color));
                    ui.label(RichText::new("Mbps").size(14.0).color(MUTED));
                });

                ui.horizontal(|ui| {
                    if snap.testing {
                        ui.spinner();
                        ui.label(t("measuring"));
                    } else if let Some(next) = snap.next_test {
                        let left = next
                            .saturating_duration_since(Instant::now())
                            .as_secs_f32()
                            .ceil() as u32;
                        ui.label(tf("next_in", &[("n", left.to_string())]));
                    }
                    if let Some(err) = &snap.last_error {
                        ui.colored_label(RED, tf("error_prefix", &[("err", err.clone())]));
                    }
                });

                ui.add_space(4.0);
                let kbs: Vec<f64> = snap.samples.iter().map(|s| s.kb_s).collect();
                let n = kbs.len();
                let avg = if n > 0 { kbs.iter().sum::<f64>() / n as f64 } else { 0.0 };
                let max = kbs.iter().copied().fold(0.0_f64, f64::max);
                let min = kbs.iter().copied().fold(f64::INFINITY, f64::min);
                let fmt_kb = |v: f64| {
                    if n > 0 {
                        format!("{v:.1} KB/s")
                    } else {
                        "-".to_string()
                    }
                };
                ui.horizontal_wrapped(|ui| {
                    kv(ui, t("avg"), fmt_kb(avg));
                    kv(ui, t("max"), fmt_kb(max));
                    kv(ui, t("min"), fmt_kb(min));
                    kv(
                        ui,
                        t("latency"),
                        latest
                            .map(|s| format!("{:.0} ms", s.latency_ms))
                            .unwrap_or_else(|| "-".into()),
                    );
                    kv(ui, t("sample_count"), n.to_string());
                });

                ui.add_space(6.0);
                region_table(ui, snap);

                ui.add_space(6.0);
                region_plot(ui, snap);

                ui.add_space(6.0);
                let pts: Vec<[f64; 2]> = snap.samples.iter().map(|s| [s.t, s.kb_s]).collect();
                Plot::new("speed_plot")
                    .height(150.0)
                    .allow_scroll(false)
                    .allow_zoom(false)
                    .allow_drag(false)
                    .x_axis_label(t("elapsed"))
                    .y_axis_label("KB/s")
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new(PlotPoints::new(pts))
                                .color(ACCENT)
                                .name(t("dl_legend")),
                        );
                    });

                ui.add_space(6.0);
                ui.label(RichText::new(t("recent_records")).strong());
                egui::Grid::new("history")
                    .striped(true)
                    .num_columns(4)
                    .spacing([18.0, 4.0])
                    .show(ui, |ui| {
                        hdr(ui, t("time"));
                        hdr(ui, "KB/s".to_string());
                        hdr(ui, "Mbps".to_string());
                        hdr(ui, t("hdr_resp"));
                        ui.end_row();
                        for s in snap.samples.iter().rev().take(8) {
                            ui.label(fmt_time(s.t));
                            ui.label(format!("{:.1}", s.kb_s));
                            ui.label(format!("{:.2}", s.mbps));
                            ui.label(format!("{:.0} ms", s.latency_ms));
                            ui.end_row();
                        }
                    });
            });
    });
}

/// 이 PC의 모든 IPv4 주소 (netinfo 기준, 없으면 스캐너의 로컬 주소)
fn my_ips(snap: &Shared) -> Vec<String> {
    let mut ips: Vec<String> = snap
        .net_info
        .as_ref()
        .map(|n| {
            n.addrs
                .iter()
                .map(|a| a.split('/').next().unwrap_or(a).to_string())
                .collect()
        })
        .unwrap_or_default();
    if let Some(ip) = snap.local_ip {
        let s = ip.to_string();
        if !ips.contains(&s) {
            ips.push(s);
        }
    }
    ips
}

fn is_my_ip(snap: &Shared, ip: Ipv4Addr) -> bool {
    my_ips(snap).contains(&ip.to_string())
}

fn prefix_to_mask(prefix: u32) -> String {
    let p = prefix.min(32);
    let mask: u32 = if p == 0 { 0 } else { u32::MAX << (32 - p) };
    Ipv4Addr::from(mask).to_string()
}

/// 원격 IP의 국가 표시 문자열 (LAN / 코드 / ? 조회 실패 / ... 조회 중)
fn country_label(snap: &Shared, ip: &str) -> String {
    if conns::is_private_ip(ip) {
        return t("lan");
    }
    match snap.conn_info.countries.get(ip) {
        Some(c) if !c.is_empty() => c.clone(),
        Some(_) => "?".to_string(),
        None => "...".to_string(),
    }
}

/// 정렬 순서: 실제 국가 코드 → 미확인 → LAN
fn country_rank(c: &str) -> u8 {
    if c == t("lan") {
        2
    } else if c == "?" || c == "..." {
        1
    } else {
        0
    }
}

/// 왼쪽 가운데: 외부 연결 포트 목록과 5초 단위 통신량
fn conn_card(ui: &mut egui::Ui, snap: &Shared) {
    card(ui, t("card_conn"), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let last = snap.traffic5.back();
                ui.label(RichText::new(t("recent5")).strong());
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t("rx_label")).color(ACCENT));
                    ui.label(
                        RichText::new(
                            last.map(|b| format!("{:.2} MB", b.rx_mb))
                                .unwrap_or_else(|| "--".into()),
                        )
                        .size(18.0)
                        .strong()
                        .color(ACCENT),
                    );
                    ui.add_space(12.0);
                    ui.label(RichText::new(t("tx_label")).color(UPLOAD));
                    ui.label(
                        RichText::new(
                            last.map(|b| format!("{:.2} MB", b.tx_mb))
                                .unwrap_or_else(|| "--".into()),
                        )
                        .size(18.0)
                        .strong()
                        .color(UPLOAD),
                    );
                });

                egui::Grid::new("traffic5")
                    .striped(true)
                    .num_columns(3)
                    .spacing([16.0, 3.0])
                    .show(ui, |ui| {
                        hdr(ui, t("time"));
                        hdr(ui, t("rx_mb"));
                        hdr(ui, t("tx_mb"));
                        ui.end_row();
                        for b in snap.traffic5.iter().rev().take(6) {
                            ui.label(fmt_time(b.t));
                            ui.label(format!("{:.2}", b.rx_mb));
                            ui.label(format!("{:.2}", b.tx_mb));
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                ui.label(
                    RichText::new(tf(
                        "conn_count",
                        &[("n", snap.conn_info.total.to_string())],
                    ))
                    .strong(),
                );
                if snap.conn_info.entries.is_empty() {
                    ui.label(RichText::new(t("no_conns")).color(MUTED));
                }
                egui::Grid::new("conns")
                    .striped(true)
                    .num_columns(5)
                    .spacing([14.0, 3.0])
                    .show(ui, |ui| {
                        hdr(ui, t("remote_ip"));
                        hdr(ui, t("country"));
                        hdr(ui, t("port"));
                        hdr(ui, t("process"));
                        hdr(ui, t("count"));
                        ui.end_row();
                        // 나라별 정렬 (국가 → 포트 순). LAN과 미확인은 맨 뒤
                        let mut sorted: Vec<(String, &conns::ConnEntry)> = snap
                            .conn_info
                            .entries
                            .iter()
                            .map(|e| (country_label(snap, &e.ip), e))
                            .collect();
                        sorted.sort_by(|a, b| {
                            country_rank(&a.0)
                                .cmp(&country_rank(&b.0))
                                .then(a.0.cmp(&b.0))
                                .then(a.1.remote_port.cmp(&b.1.remote_port))
                        });
                        for (country, e) in sorted {
                            let svc = conns::port_name(e.remote_port);
                            let port = if svc.is_empty() {
                                e.remote_port.to_string()
                            } else {
                                format!("{} ({})", e.remote_port, svc)
                            };
                            let process = if e.process.is_empty() {
                                t("unknown")
                            } else {
                                e.process.clone()
                            };
                            ui.label(RichText::new(&e.ip).monospace());
                            ui.label(RichText::new(country).strong());
                            ui.label(port);
                            ui.label(process);
                            ui.label(e.count.to_string());
                            ui.end_row();
                        }
                    });

                ui.add_space(8.0);
                ui.label(RichText::new(t("port_totals_title")).strong());
                if let Some((key, code)) = snap.conn_info.note {
                    ui.colored_label(YELLOW, tf(key, &[("n", code.to_string())]));
                }
                let mut totals: Vec<(u16, u64, u64)> = snap
                    .conn_info
                    .port_totals
                    .iter()
                    .map(|(p, (rx, tx))| (*p, *rx, *tx))
                    .collect();
                totals.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)));
                if totals.is_empty() {
                    ui.label(RichText::new(t("no_totals")).color(MUTED));
                }
                egui::Grid::new("port_totals")
                    .striped(true)
                    .num_columns(4)
                    .spacing([14.0, 3.0])
                    .show(ui, |ui| {
                        hdr(ui, t("port"));
                        hdr(ui, t("service"));
                        hdr(ui, t("total_rx"));
                        hdr(ui, t("total_tx"));
                        ui.end_row();
                        for (port, rx, tx) in totals.iter().take(12) {
                            let svc = conns::port_name(*port);
                            ui.label(port.to_string());
                            ui.label(if svc.is_empty() { "-" } else { svc });
                            ui.label(RichText::new(fmt_bytes(*rx)).color(ACCENT));
                            ui.label(RichText::new(fmt_bytes(*tx)).color(UPLOAD));
                            ui.end_row();
                        }
                    });

                ui.add_space(6.0);
                ui.label(RichText::new(t("conn_footnote")).color(MUTED).small());
            });
    });
}

/// 왼쪽 아래: 내 공유 폴더 목록과 접속 IP/이름
fn share_card(ui: &mut egui::Ui, snap: &Shared) {
    let info = &snap.share_info;
    card(ui, t("card_share"), |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if let Some(err) = &info.error {
                    ui.colored_label(RED, err.as_str());
                }

                ui.label(
                    RichText::new(tf("shares_count", &[("n", info.shares.len().to_string())]))
                        .strong(),
                );
                if info.shares.is_empty() {
                    ui.label(RichText::new(t("no_shares")).color(MUTED));
                }
                for s in &info.shares {
                    ui.label(format!("{}  :  {}", s.name, s.path));
                }

                ui.add_space(8.0);
                let color = if info.peers.is_empty() { MUTED } else { GREEN };
                ui.label(
                    RichText::new(tf("peers_count", &[("n", info.peers.len().to_string())]))
                        .strong()
                        .color(color),
                );
                if info.peers.is_empty() {
                    ui.label(RichText::new(t("no_peers")).color(MUTED));
                }
                for p in &info.peers {
                    let name = if p.name.is_empty() {
                        t("no_name")
                    } else {
                        p.name.clone()
                    };
                    ui.label(format!("{}  :  {}", p.ip, name));
                }

                ui.add_space(6.0);
                ui.label(RichText::new(t("share_footnote")).color(MUTED).small());
            });
    });
}

fn net_card(ui: &mut egui::Ui, snap: &Shared) {
    card(ui, t("card_net"), |ui| {
        let Some(n) = &snap.net_info else {
            ui.label(RichText::new(t("net_loading")).color(MUTED));
            return;
        };

        let mask = n
            .prefix
            .parse::<u32>()
            .ok()
            .map(prefix_to_mask)
            .unwrap_or_else(|| "-".to_string());
        let dns = if n.dns.is_empty() {
            "-".to_string()
        } else {
            n.dns.join(", ")
        };
        let ip_list = if n.addrs.is_empty() {
            n.ipv4.clone()
        } else {
            n.addrs.join("\n")
        };
        let kind = if n.media.contains("802.11") {
            "Wi-Fi".to_string()
        } else {
            t("wired")
        };
        let assign = if n.dhcp.eq_ignore_ascii_case("Enabled") {
            t("dhcp_auto")
        } else {
            t("dhcp_manual")
        };
        let speed_now = snap
            .samples
            .back()
            .map(|s| format!("{:.1} KB/s ({:.2} Mbps)", s.kb_s, s.mbps))
            .unwrap_or_else(|| t("waiting"));

        let mut rows: Vec<(String, String)> = vec![
            (t("conn_type"), kind),
            (t("adapter"), n.adapter.clone()),
            (t("description"), n.description.clone()),
            (t("ipv4_all"), ip_list),
            (t("subnet"), mask),
            (t("gateway"), n.gateway.clone()),
            (t("dns"), dns),
            (t("ip_assign"), assign),
            (t("mac"), n.mac.clone()),
            (t("link_speed"), n.link_speed.clone()),
            ("MTU".to_string(), n.mtu.clone()),
        ];
        if let Some(ssid) = &n.ssid {
            rows.push(("Wi-Fi SSID".to_string(), ssid.clone()));
        }
        if let Some(sig) = &n.signal {
            rows.push((t("signal"), sig.clone()));
        }
        if let Some(rate) = &n.wifi_rate {
            rows.push((t("rx_link_speed"), rate.clone()));
        }
        rows.push((t("current_dl"), speed_now));

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("netcfg")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .striped(true)
                    .show(ui, |ui| {
                        for (k, v) in rows {
                            ui.label(RichText::new(k).color(MUTED));
                            ui.label(RichText::new(v).strong());
                            ui.end_row();
                        }
                    });
                ui.add_space(6.0);
                ui.label(RichText::new(t("net_refresh_note")).color(MUTED).small());
            });
    });
}

/// 언어 선택 (왼쪽 상단)
fn language_picker(ui: &mut egui::Ui) {
    let mut cur = i18n::current();
    egui::ComboBox::from_id_salt("lang_select")
        .selected_text(cur.label())
        .show_ui(ui, |ui| {
            for l in i18n::LANGS {
                ui.selectable_value(&mut cur, l, l.label());
            }
        });
    ui.label(RichText::new(t("language")).color(MUTED));
    if cur != i18n::current() {
        i18n::set_lang(cur);
    }
}

impl eframe::App for NetsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(250));

        let snap = match self.shared.lock() {
            Ok(guard) => guard.clone(),
            Err(_) => return,
        };

        // 언어가 바뀌면 창 제목도 바꿈
        let lang = i18n::current();
        if self.title_lang != Some(lang) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(t("app_title")));
            self.title_lang = Some(lang);
        }

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // "MRPUZZLE" 는 빨간색, "-NETS" 는 강조색
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.heading(RichText::new("MRPUZZLE").color(RED).strong());
                ui.heading(RichText::new("-NETS").color(ACCENT).strong());
                ui.add_space(8.0);
                ui.label(RichText::new(t("net_monitor")).color(MUTED));
                ui.separator();
                let ips = my_ips(&snap);
                let ip = if ips.is_empty() {
                    "-".to_string()
                } else {
                    ips.join(", ")
                };
                ui.label(tf("my_ip", &[("ip", ip)]));
                ui.separator();
                if snap.scanning {
                    ui.spinner();
                }
                let status = if snap.scanning {
                    t("scanning")
                } else if snap.last_scan.is_none() {
                    t("not_scanned")
                } else if snap.local_ip.is_none() {
                    t("no_network")
                } else {
                    tf("found_n", &[("n", snap.devices.len().to_string())])
                };
                ui.label(status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 오른쪽 끝: 언어 선택 (오른쪽→왼쪽 배치이므로 먼저 그림)
                    language_picker(ui);
                    ui.separator();
                    if ui.button(t("help")).clicked() {
                        self.show_help = !self.show_help;
                    }
                    if ui
                        .add_enabled(!snap.scanning, egui::Button::new(t("scan_now")))
                        .clicked()
                    {
                        let _ = self.rescan_tx.send(());
                    }
                });
            });
            ui.add_space(6.0);
        });

        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let scanned = match snap.last_scan {
                    Some(t0) => tf(
                        "last_scan_ago",
                        &[("s", t0.elapsed().as_secs().to_string())],
                    ),
                    None => t("not_scanned"),
                };
                ui.label(RichText::new(scanned).color(MUTED).small());
                ui.label(RichText::new(t("auto_refresh_60")).color(MUTED).small());
            });
        });

        // 초기 화면 비율: 왼쪽 3 : 가운데 2 : 오른쪽 5 (저장된 배치가 있으면 그 값 사용)
        let total_w = ctx.screen_rect().width();
        let init = self.initial;

        let left_resp = egui::SidePanel::left("netcfg_panel")
            .default_width(init.left.unwrap_or(total_w * 0.3))
            .min_width(200.0)
            .show(ctx, |ui| {
                let share = egui::TopBottomPanel::bottom("share_panel")
                    .resizable(true)
                    .default_height(init.share_h.unwrap_or(260.0))
                    .show_inside(ui, |ui| {
                        share_card(ui, &snap);
                    });
                let net = egui::TopBottomPanel::top("netcfg_top")
                    .resizable(true)
                    .default_height(init.net_h.unwrap_or(300.0))
                    .show_inside(ui, |ui| {
                        net_card(ui, &snap);
                    });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    conn_card(ui, &snap);
                });
                (share.response.rect.height(), net.response.rect.height())
            });
        let (share_h, net_h) = left_resp.inner;
        let left_w = left_resp.response.rect.width();

        let right_resp = egui::SidePanel::right("speed_panel")
            .default_width(init.right.unwrap_or(total_w * 0.5))
            .min_width(300.0)
            .show(ctx, |ui| {
                speed_card(ui, &snap);
            });
        let right_w = right_resp.response.rect.width();

        egui::CentralPanel::default().show(ctx, |ui| {
            devices_card(ui, &snap);
        });

        // 현재 배치와 언어를 읽어 바뀌었을 때만 저장 (1px 단위로 반올림)
        let r1 = |v: f32| v.round();
        let win = ctx
            .input(|i| i.viewport().inner_rect)
            .map(|r| [r1(r.width()), r1(r.height())]);
        let pos = ctx
            .input(|i| i.viewport().outer_rect)
            .map(|r| [r1(r.min.x), r1(r.min.y)]);
        let cur = settings::Layout {
            win,
            pos,
            left: Some(r1(left_w)),
            right: Some(r1(right_w)),
            net_h: Some(r1(net_h)),
            share_h: Some(r1(share_h)),
            lang: Some(i18n::current().index() as u8),
        };
        if cur != self.saved {
            settings::save(&cur);
            self.saved = cur;
        }

        // 도움말 창
        egui::Window::new(t("help_title"))
            .open(&mut self.show_help)
            .resizable(true)
            .default_width(520.0)
            .default_height(560.0)
            .show(ctx, |ui| {
                help_contents(ui);
            });
    }
}

/// 도움말 내용: 사용법, 무료 사용 안내, 제작자, 사용 라이선스
fn help_contents(ui: &mut egui::Ui) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.heading(RichText::new(t("help_heading")).color(ACCENT).strong());
            ui.label(tf(
                "help_version",
                &[("v", env!("CARGO_PKG_VERSION").to_string())],
            ));
            ui.add_space(6.0);
            ui.label(RichText::new(t("help_author")).strong());
            ui.label(RichText::new(t("help_free")).strong());
            ui.label(t("help_free_modify"));
            ui.label(RichText::new(t("help_attrib")).strong());
            ui.add_space(10.0);

            ui.label(RichText::new(t("help_screens")).color(ACCENT).strong());
            ui.label(t("help_s1"));
            ui.label(t("help_s2"));
            ui.label(t("help_s3"));
            ui.label(t("help_s4"));
            ui.label(t("help_s5"));
            ui.add_space(6.0);
            ui.label(t("help_s6"));
            ui.label(t("help_s7"));
            ui.label(t("help_s8"));
            ui.add_space(10.0);

            ui.label(RichText::new(t("help_runtime")).color(ACCENT).strong());
            ui.label(t("help_r1"));
            ui.label(t("help_r2"));
            ui.label(t("help_r3"));
            ui.label(t("help_r4"));
            ui.add_space(10.0);

            ui.label(RichText::new(t("help_license")).color(ACCENT).strong());
            ui.label(RichText::new(t("help_license_name")).strong());
            ui.label(t("lic_rust"));
            ui.label(t("lic_egui"));
            ui.label(t("lic_plot"));
            ui.label(t("lic_ureq"));
            ui.label(t("lic_font"));
            ui.label(t("lic_services"));
            ui.add_space(6.0);
            ui.label(RichText::new(t("lic_note")).color(MUTED).small());
            ui.add_space(8.0);
            ui.label(RichText::new("© MRPUZZLE").color(MUTED));
        });
}

fn main() -> eframe::Result<()> {
    // 저장된 배치와 언어 불러오기 (없으면 기본값)
    let saved = settings::load();
    i18n::set_lang(Lang::from_index(saved.lang.unwrap_or(0)));

    let mut viewport = egui::ViewportBuilder::default()
        .with_title(t("app_title"))
        .with_inner_size(saved.win.unwrap_or([1500.0, 800.0]))
        .with_min_inner_size([760.0, 480.0]);
    if let Some([x, y]) = saved.pos {
        viewport = viewport.with_position([x, y]);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "MRPUZZLE-NETS",
        options,
        Box::new(move |cc| Ok(Box::new(NetsApp::new(cc, saved)))),
    )
}
