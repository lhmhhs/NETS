//! 주변 네트워크 장치 탐색 모듈
//! - 내 PC의 IPv4 주소로 같은 /24 대역에 ping 스윕 (`ping -a`로 호스트 이름도 조회)
//! - `arp -a` 캐시에서 MAC 주소를 읽어 장치 정보를 보강하고 제조사를 추정

use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::process::Command;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::oui;
use crate::Shared;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 자동 재스캔 주기
const RESCAN_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
pub struct Device {
    pub ip: Ipv4Addr,
    pub mac: Option<String>,
    /// 호스트 이름 (역방향 DNS, ping -a로 조회)
    pub hostname: Option<String>,
    /// MAC 앞 6자리로 추정한 제조사/분류
    pub vendor: Option<&'static str>,
    pub latency_ms: Option<u32>,
    pub by_ping: bool,
    pub by_arp: bool,
}

/// 콘솔 창이 깜빡이지 않도록 외부 명령을 숨김 실행
fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 외부로 라우팅되는 로컬 IPv4 주소 (실제 패킷은 보내지 않음)
pub fn local_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()?.ip() {
        IpAddr::V4(v4) if !v4.is_loopback() && !v4.is_unspecified() => Some(v4),
        _ => None,
    }
}

/// ping 한 번 수행. 응답이 있으면 (응답시간 ms, 호스트 이름)을 돌려줌
fn ping(ip: Ipv4Addr) -> Option<(u32, Option<String>)> {
    let out = hidden(Command::new("ping").args([
        "-a",
        "-n",
        "1",
        "-w",
        "700",
        &ip.to_string(),
    ]))
    .output()
    .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let latency = parse_ms(&text).unwrap_or(0);
    Some((latency, parse_hostname(&text, ip)))
}

/// "time=12ms", "시간<1ms" 같은 출력에서 숫자 추출
fn parse_ms(text: &str) -> Option<u32> {
    for line in text.lines() {
        if let Some(pos) = line.find("ms") {
            let before = &line[..pos];
            let digits_start = before.trim_end_matches(|c: char| c.is_ascii_digit()).len();
            let prefix = &before[..digits_start];
            if prefix.ends_with('=') || prefix.ends_with('<') {
                if let Ok(v) = before[digits_start..].parse::<u32>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

/// `ping -a` 첫 줄의 "이름 [IP]" 형태에서 호스트 이름 추출 (언어와 무관하게 "[IP]" 바로 앞 토큰)
fn parse_hostname(text: &str, ip: Ipv4Addr) -> Option<String> {
    let needle = format!("[{ip}]");
    for line in text.lines() {
        if let Some(pos) = line.find(&needle) {
            let before = line[..pos].trim_end();
            let name = before.split_whitespace().last()?;
            if name != ip.to_string() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// `arp -a` 출력에서 (IP, MAC) 목록 읽기
fn read_arp() -> Vec<(Ipv4Addr, String)> {
    let mut result = Vec::new();
    let Ok(out) = hidden(Command::new("arp").arg("-a")).output() else {
        return result;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let mut it = line.split_whitespace();
        let (Some(ip_s), Some(mac_s)) = (it.next(), it.next()) else {
            continue;
        };
        let Ok(ip) = ip_s.parse::<Ipv4Addr>() else {
            continue;
        };
        if mac_s.len() == 17 && mac_s.matches('-').count() == 5 {
            result.push((ip, mac_s.to_uppercase()));
        }
    }
    result
}

/// 한 번의 전체 스캔
pub fn scan_once() -> (Option<Ipv4Addr>, Vec<Device>) {
    let Some(local) = local_ipv4() else {
        return (None, Vec::new());
    };
    let [a, b, c, _] = local.octets();

    // 1) 같은 /24 대역 ping 스윕 (병렬, 호스트 이름 포함)
    let handles: Vec<_> = (1..=254u8)
        .map(|i| {
            let ip = Ipv4Addr::new(a, b, c, i);
            thread::spawn(move || (ip, ping(ip)))
        })
        .collect();

    let mut map: BTreeMap<Ipv4Addr, Device> = BTreeMap::new();
    for h in handles {
        if let Ok((ip, Some((lat, host)))) = h.join() {
            map.insert(
                ip,
                Device {
                    ip,
                    mac: None,
                    hostname: host,
                    vendor: None,
                    latency_ms: Some(lat),
                    by_ping: true,
                    by_arp: false,
                },
            );
        }
    }

    // 2) ARP 캐시로 MAC 보강 (ping이 막힌 장치도 캐시에 있으면 표시)
    for (ip, mac) in read_arp() {
        let o = ip.octets();
        if o[..3] != [a, b, c] || o[3] == 255 || o[0] >= 224 {
            continue;
        }
        let e = map.entry(ip).or_insert(Device {
            ip,
            mac: None,
            hostname: None,
            vendor: None,
            latency_ms: None,
            by_ping: false,
            by_arp: true,
        });
        e.mac = Some(mac);
        e.by_arp = true;
    }

    // 3) 내 PC도 목록에 포함 (이름은 컴퓨터 이름 환경변수 사용)
    map.entry(local).or_insert(Device {
        ip: local,
        mac: None,
        hostname: std::env::var("COMPUTERNAME").ok(),
        vendor: None,
        latency_ms: Some(0),
        by_ping: false,
        by_arp: false,
    });

    // 4) MAC으로 제조사 추정
    for d in map.values_mut() {
        d.vendor = d.mac.as_deref().and_then(oui::lookup);
    }

    (Some(local), map.into_values().collect())
}

/// 스캐너 스레드: 주기적으로 스캔하고, 버튼 요청(rx)이 오면 즉시 스캔
pub fn run(shared: Arc<Mutex<Shared>>, rx: Receiver<()>) {
    loop {
        if let Ok(mut s) = shared.lock() {
            s.scanning = true;
        }

        let (local, devices) = scan_once();

        if let Ok(mut s) = shared.lock() {
            s.scanning = false;
            s.last_scan = Some(Instant::now());
            s.local_ip = local;
            s.devices = devices;
        }

        match rx.recv_timeout(RESCAN_INTERVAL) {
            Ok(()) | Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}
