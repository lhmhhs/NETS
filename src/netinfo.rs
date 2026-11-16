//! 내 네트워크 설정 조회 모듈
//! 기본 게이트웨이를 가진 활성 어댑터의 IP, 게이트웨이, DNS, MAC, 링크 속도 등을 읽습니다.
//! Wi-Fi 연결이면 `netsh wlan`으로 SSID, 신호 강도, 수신 속도도 읽습니다.

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::Shared;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 설정 갱신 주기
const REFRESH_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Default)]
pub struct NetInfo {
    pub adapter: String,
    pub description: String,
    pub media: String,
    pub mac: String,
    pub link_speed: String,
    pub ipv4: String,
    pub prefix: String,
    pub gateway: String,
    pub dns: Vec<String>,
    pub dhcp: String,
    pub mtu: String,
    pub ssid: Option<String>,
    pub signal: Option<String>,
    pub wifi_rate: Option<String>,
    /// 이 PC의 모든 IPv4 주소 ("192.168.0.10/24 [이더넷]" 형식)
    pub addrs: Vec<String>,
}

fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 기본 게이트웨이가 있는 활성 어댑터 정보를 PowerShell로 조회 (언어와 무관한 key=value 형식)
const PS_SCRIPT: &str = r##"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Get-NetIPAddress -AddressFamily IPv4 | Where-Object { $_.AddressState -eq 'Preferred' -and $_.IPAddress -notlike '127.*' } | ForEach-Object { "addr=$($_.IPAddress)/$($_.PrefixLength) [$($_.InterfaceAlias)]" }
$c = Get-NetIPConfiguration | Where-Object { $_.IPv4DefaultGateway -and $_.NetAdapter.Status -eq 'Up' } | Select-Object -First 1
if (-not $c) { exit 0 }
$a = $c.NetAdapter
$ipif = Get-NetIPInterface -InterfaceIndex $a.ifIndex -AddressFamily IPv4
"adapter=$($a.Name)"
"desc=$($a.InterfaceDescription)"
"media=$($a.MediaType)"
"mac=$($a.MacAddress)"
"speed=$($a.LinkSpeed)"
"ipv4=$($c.IPv4Address[0].IPAddress)"
"prefix=$($c.IPv4Address[0].PrefixLength)"
"gw=$($c.IPv4DefaultGateway.NextHop)"
"dns=$(($c.DNSServer.ServerAddresses) -join ',')"
"dhcp=$($ipif.Dhcp)"
"mtu=$($ipif.NlMtu)"
"##;

/// Wi-Fi 상태 (SSID, 신호 강도, 수신 속도). 키 이름은 언어에 따라 다르므로 값의 형태로 판별
fn read_wifi() -> (Option<String>, Option<String>, Option<String>) {
    let Ok(out) = hidden(Command::new("netsh").args(["wlan", "show", "interfaces"])).output()
    else {
        return (None, None, None);
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut ssid = None;
    let mut signal = None;
    let mut rate = None;
    for line in text.lines() {
        let Some((k, v)) = line.trim().split_once(':') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        if k == "SSID" && !v.is_empty() {
            ssid = Some(v.to_string());
        } else if v.ends_with('%') && signal.is_none() {
            signal = Some(v.to_string());
        } else if v.ends_with("Mbps") && rate.is_none() {
            // 첫 번째 Mbps 값이 수신 속도
            rate = Some(v.to_string());
        }
    }
    (ssid, signal, rate)
}

/// 현재 네트워크 설정 조회
pub fn query() -> Option<NetInfo> {
    let out = hidden(Command::new("powershell").args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        PS_SCRIPT,
    ]))
    .output()
    .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);

    let mut map: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        if let Some((k, v)) = line.trim().split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    if map.is_empty() {
        return None;
    }

    let addrs: Vec<String> = text
        .lines()
        .filter_map(|l| l.trim().strip_prefix("addr=").map(|s| s.to_string()))
        .collect();

    let get = |k: &str| map.get(k).cloned().unwrap_or_default();
    let media = get("media");
    let is_wifi = media.contains("802.11");
    let (ssid, signal, wifi_rate) = if is_wifi {
        read_wifi()
    } else {
        (None, None, None)
    };

    let dns = get("dns")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Some(NetInfo {
        adapter: get("adapter"),
        description: get("desc"),
        media,
        mac: get("mac"),
        link_speed: get("speed"),
        ipv4: get("ipv4"),
        prefix: get("prefix"),
        gateway: get("gw"),
        dns,
        dhcp: get("dhcp"),
        mtu: get("mtu"),
        ssid,
        signal,
        wifi_rate,
        addrs,
    })
}

/// 설정 스레드: 30초마다 갱신
pub fn run(shared: Arc<Mutex<Shared>>) {
    loop {
        let info = query();
        if let Ok(mut s) = shared.lock() {
            s.net_info = info;
        }
        thread::sleep(REFRESH_INTERVAL);
    }
}
