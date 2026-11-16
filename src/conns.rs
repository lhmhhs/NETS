//! 외부 연결 목록 + 포트별 누적 통신량 모듈
//! - 연결 목록: Get-NetTCPConnection (PowerShell)
//! - 연결별 누적 바이트: iphlpapi TCP EStats (관리자 권한 필요, 프로그램은 관리자로 실행됨)
//! - 국가 코드: ipinfo.io 로 원격 IP 조회 (IP별 1회, 캐시)

use std::collections::{BTreeMap, HashMap, HashSet};
use std::mem::size_of;
use std::net::{IpAddr, Ipv4Addr};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::Shared;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 갱신 주기 (5초)
const REFRESH_INTERVAL: Duration = Duration::from_secs(5);

/// (로컬IP, 로컬포트, 원격IP, 원격포트)
type ConnKey = (String, u16, String, u16);

#[derive(Clone, Debug)]
pub struct RawConn {
    pub lip: String,
    pub lport: u16,
    pub rip: String,
    pub rport: u16,
    pub process: String,
}

#[derive(Clone, Debug)]
pub struct ConnEntry {
    pub ip: String,
    pub remote_port: u16,
    pub process: String,
    /// 같은 IP/포트/프로세스의 연결 개수
    pub count: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ConnInfo {
    pub entries: Vec<ConnEntry>,
    /// 전체 외부 연결 수
    pub total: usize,
    /// 원격 IP → 국가 코드 (KR, US 등). 조회 실패는 빈 문자열
    pub countries: HashMap<String, String>,
    /// 원격 포트별 누적 (수신, 송신) 바이트 — NETS 실행 이후 집계
    pub port_totals: BTreeMap<u16, (u64, u64)>,
    /// 통계 조회 안내: (번역 키, 오류 코드)
    pub note: Option<(&'static str, u32)>,
}

/// 사설/로컬 주소인지 (LAN, 루프백, 링크 로컬 등)
pub fn is_private_ip(s: &str) -> bool {
    match s.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => {
            v4.is_private() || v4.is_loopback() || v4.is_link_local() || v4.is_unspecified()
        }
        Ok(IpAddr::V6(v6)) => {
            let seg = v6.segments()[0];
            v6.is_loopback() || (seg & 0xfe00) == 0xfc00 || (seg & 0xffc0) == 0xfe80
        }
        Err(_) => true,
    }
}

/// IP의 국가 코드 조회 (ipinfo.io, 2자리 코드)
fn country_of(ip: &str) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();
    let body = agent
        .get(&format!("https://ipinfo.io/{ip}/country"))
        .call()
        .ok()?
        .into_string()
        .ok()?;
    let code = body.trim();
    if code.len() == 2 && code.chars().all(|c| c.is_ascii_alphabetic()) {
        Some(code.to_uppercase())
    } else {
        None
    }
}

fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 자주 쓰는 포트의 서비스 이름
pub fn port_name(port: u16) -> &'static str {
    match port {
        20 | 21 => "FTP",
        22 => "SSH",
        23 => "Telnet",
        25 | 465 | 587 => "SMTP",
        53 => "DNS",
        80 => "HTTP",
        110 | 995 => "POP3",
        143 | 993 => "IMAP",
        443 => "HTTPS",
        445 => "SMB",
        853 => "DNS-over-TLS",
        3389 => "RDP",
        5222 => "XMPP/메신저",
        5228 => "Google 푸시",
        8080 => "HTTP(대체)",
        8443 => "HTTPS(대체)",
        _ => "",
    }
}

/// 외부 TCP 연결 목록을 "conn=로컬IP|로컬포트|원격IP|원격포트|프로세스" 줄로 출력
const PS_SCRIPT: &str = r##"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$procs = @{}
Get-Process -ErrorAction SilentlyContinue | ForEach-Object { $procs[[int]$_.Id] = $_.ProcessName }
Get-NetTCPConnection -State Established -ErrorAction SilentlyContinue |
  Where-Object { $_.RemoteAddress -notmatch '^(127\.|::1$|0\.0\.0\.0$)' } |
  ForEach-Object {
    $p = $procs[[int]$_.OwningProcess]
    "conn=$($_.LocalAddress)|$($_.LocalPort)|$($_.RemoteAddress)|$($_.RemotePort)|$p"
  }
"##;

fn query_raw() -> Vec<RawConn> {
    let Ok(out) = hidden(Command::new("powershell").args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        PS_SCRIPT,
    ]))
    .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);

    let mut conns = Vec::new();
    for line in text.lines() {
        let Some(v) = line.trim().strip_prefix("conn=") else {
            continue;
        };
        let parts: Vec<&str> = v.splitn(5, '|').collect();
        if parts.len() < 4 {
            continue;
        }
        let (Ok(lport), Ok(rport)) = (parts[1].parse::<u16>(), parts[3].parse::<u16>()) else {
            continue;
        };
        let process = match parts.get(4).map(|s| s.trim()) {
            Some(p) if !p.is_empty() => p.to_string(),
            // 이름을 못 찾으면 빈 값 (화면에서 언어에 맞게 표시)
            _ => String::new(),
        };
        conns.push(RawConn {
            lip: parts[0].to_string(),
            lport,
            rip: parts[2].to_string(),
            rport,
            process,
        });
    }
    conns
}

// ---------- iphlpapi TCP EStats (연결별 누적 바이트) ----------

/// MIB_TCPROW: 주소/포트는 네트워크 바이트 순서
#[repr(C)]
#[derive(Clone, Copy)]
struct MibTcpRow {
    state: u32,
    local_addr: u32,
    local_port: u32,
    remote_addr: u32,
    remote_port: u32,
}

/// TCP_ESTATS_DATA_ROD_v0 (읽기 전용 누적 데이터). 앞쪽 필드만 사용
#[repr(C)]
#[derive(Default)]
struct EstatsRodV0 {
    data_bytes_out: u64,
    data_segs_out: u64,
    data_bytes_in: u64,
    data_segs_in: u64,
    segs_out: u64,
    segs_in: u64,
    soft_errors: u32,
    soft_error_reason: u32,
    snd_una: u32,
    snd_nxt: u32,
    snd_max: u32,
    thru_bytes_acked: u64,
    rcv_nxt: u32,
    thru_bytes_received: u64,
}

const TCP_ESTATS_DATA: u32 = 1;
const TCP_ESTATS_ROD: u32 = 9;
const MIB_TCP_STATE_ESTAB: u32 = 5;

#[link(name = "iphlpapi")]
extern "system" {
    fn SetPerTcpConnectionEStats(
        row: *mut MibTcpRow,
        stats_type: u32,
        rw: *const u8,
        rw_version: u32,
        rw_size: u32,
        offset: u32,
    ) -> u32;
    fn GetPerTcpConnectionEStats(
        row: *mut MibTcpRow,
        stats_type: u32,
        rw: *mut u8,
        rw_version: u32,
        rw_size: u32,
        ros: *mut u8,
        ros_version: u32,
        ros_size: u32,
        rod: *mut u8,
        rod_version: u32,
        rod_size: u32,
    ) -> u32;
}

fn make_row(key: &ConnKey) -> Option<MibTcpRow> {
    let lip: Ipv4Addr = key.0.parse().ok()?;
    let rip: Ipv4Addr = key.2.parse().ok()?;
    Some(MibTcpRow {
        state: MIB_TCP_STATE_ESTAB,
        local_addr: u32::from_ne_bytes(lip.octets()),
        local_port: key.1.to_be() as u32,
        remote_addr: u32::from_ne_bytes(rip.octets()),
        remote_port: key.3.to_be() as u32,
    })
}

/// 연결별 마지막 누적값과 포트별 합계를 기억하는 추적기
#[derive(Default)]
struct Tracker {
    enabled: HashSet<ConnKey>,
    last: HashMap<ConnKey, (u64, u64)>,
    totals: BTreeMap<u16, (u64, u64)>,
}

/// 현재 연결들의 누적 바이트를 읽어 포트별 합계를 갱신. 실패 시 안내 문구 반환
fn update_tracker(tr: &mut Tracker, conns: &[RawConn]) -> Option<(&'static str, u32)> {
    let mut note = None;
    let mut alive: HashSet<ConnKey> = HashSet::new();

    for c in conns {
        let key: ConnKey = (c.lip.clone(), c.lport, c.rip.clone(), c.rport);
        // IPv4 연결만 EStats 대상
        let Some(mut row) = make_row(&key) else {
            continue;
        };
        alive.insert(key.clone());

        if !tr.enabled.contains(&key) {
            let rw = [1u8];
            let code = unsafe {
                SetPerTcpConnectionEStats(&mut row, TCP_ESTATS_DATA, rw.as_ptr(), 0, 1, 0)
            };
            if code != 0 {
                note = Some(("note_enable_fail", code));
                continue;
            }
            tr.enabled.insert(key.clone());
        }

        let mut rod = EstatsRodV0::default();
        let code = unsafe {
            GetPerTcpConnectionEStats(
                &mut row,
                TCP_ESTATS_ROD,
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null_mut(),
                0,
                0,
                &mut rod as *mut EstatsRodV0 as *mut u8,
                0,
                size_of::<EstatsRodV0>() as u32,
            )
        };
        if code != 0 {
            note = Some(("note_read_fail", code));
            continue;
        }

        let (bin, bout) = (rod.data_bytes_in, rod.data_bytes_out);
        let (lin, lout) = tr.last.get(&key).copied().unwrap_or((0, 0));
        // 카운터가 줄어들었으면 연결이 새로 시작된 것으로 보고 현재값을 그대로 더함
        let din = if bin >= lin { bin - lin } else { bin };
        let dout = if bout >= lout { bout - lout } else { bout };
        let total = tr.totals.entry(c.rport).or_insert((0, 0));
        total.0 += din;
        total.1 += dout;
        tr.last.insert(key, (bin, bout));
    }

    // 끊어진 연결은 추적 목록에서 제거 (포트별 합계는 유지)
    tr.enabled.retain(|k| alive.contains(k));
    tr.last.retain(|k, _| alive.contains(k));
    note
}

/// 원격 IP + 포트 + 프로세스로 묶은 목록
fn build_entries(conns: &[RawConn]) -> Vec<ConnEntry> {
    let mut groups: BTreeMap<(String, u16, String), usize> = BTreeMap::new();
    for c in conns {
        *groups
            .entry((c.rip.clone(), c.rport, c.process.clone()))
            .or_insert(0) += 1;
    }
    let mut entries: Vec<ConnEntry> = groups
        .into_iter()
        .map(|((ip, remote_port, process), count)| ConnEntry {
            ip,
            remote_port,
            process,
            count,
        })
        .collect();
    entries.sort_by(|a, b| b.count.cmp(&a.count).then(a.remote_port.cmp(&b.remote_port)));
    entries
}

/// 갱신 스레드: 5초마다 연결 목록, 누적 통신량, 국가 코드를 공유 상태에 기록
pub fn run(shared: Arc<Mutex<Shared>>) {
    // 한 번 조회한 IP는 다시 묻지 않도록 캐시 (실패도 빈 문자열로 저장)
    let mut cache: HashMap<String, String> = HashMap::new();
    let mut tracker = Tracker::default();
    loop {
        let conns = query_raw();
        let note = update_tracker(&mut tracker, &conns);
        let entries = build_entries(&conns);

        let mut info = ConnInfo {
            total: conns.len(),
            entries,
            port_totals: tracker.totals.clone(),
            note,
            countries: HashMap::new(),
        };
        if let Ok(mut s) = shared.lock() {
            s.conn_info = info.clone();
        }

        for e in &info.entries {
            if !is_private_ip(&e.ip) && !cache.contains_key(&e.ip) {
                let code = country_of(&e.ip).unwrap_or_default();
                cache.insert(e.ip.clone(), code);
            }
        }
        info.countries = cache.clone();
        if let Ok(mut s) = shared.lock() {
            s.conn_info = info;
        }

        thread::sleep(REFRESH_INTERVAL);
    }
}
