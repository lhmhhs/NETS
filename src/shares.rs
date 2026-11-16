//! 공유 폴더 / 접속 현황 모듈
//! - 이 PC가 공유 중인 폴더 목록 (Get-SmbShare, 관리자 권한 불필요)
//! - 이 PC의 SMB(445) 포트에 연결된 상대 IP와 호스트 이름 (Get-NetTCPConnection)

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::Shared;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 갱신 주기
const REFRESH_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Clone, Debug)]
pub struct ShareEntry {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug)]
pub struct PeerEntry {
    pub ip: String,
    /// 역방향 DNS로 얻은 호스트 이름 (없으면 빈 문자열)
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct ShareInfo {
    pub shares: Vec<ShareEntry>,
    pub peers: Vec<PeerEntry>,
    pub error: Option<String>,
}

fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// 공유 목록과 접속 IP/이름을 "key=value" 줄로 출력하는 PowerShell 스크립트
const PS_SCRIPT: &str = r##"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
Get-SmbShare -ErrorAction SilentlyContinue | ForEach-Object { "share=$($_.Name)|$($_.Path)" }
$ips = Get-NetTCPConnection -LocalPort 445 -State Established -ErrorAction SilentlyContinue | Select-Object -ExpandProperty RemoteAddress | Sort-Object -Unique
foreach ($ip in $ips) {
  $n = ''
  try { $n = [System.Net.Dns]::GetHostEntry($ip).HostName } catch {}
  "peer=$ip|$n"
}
"##;

/// 공유 폴더와 접속 목록 조회 (숨김 공유 $ 는 제외)
pub fn query() -> ShareInfo {
    let out = match hidden(Command::new("powershell").args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        PS_SCRIPT,
    ]))
    .output()
    {
        Ok(o) => o,
        Err(e) => {
            return ShareInfo {
                error: Some(format!("조회 실패: {e}")),
                ..Default::default()
            }
        }
    };
    let text = String::from_utf8_lossy(&out.stdout);

    let mut info = ShareInfo::default();
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("share=") {
            let mut parts = v.splitn(2, '|');
            let name = parts.next().unwrap_or("").to_string();
            let path = parts.next().unwrap_or("").to_string();
            // 관리자용 숨김 공유(C$, ADMIN$, IPC$ 등)는 제외
            if !name.is_empty() && !name.ends_with('$') {
                info.shares.push(ShareEntry { name, path });
            }
        } else if let Some(v) = line.strip_prefix("peer=") {
            let mut parts = v.splitn(2, '|');
            let ip = parts.next().unwrap_or("").to_string();
            let name = parts.next().unwrap_or("").to_string();
            if !ip.is_empty() {
                info.peers.push(PeerEntry { ip, name });
            }
        }
    }
    info
}

/// 갱신 스레드: 주기적으로 공유 목록과 접속 현황을 공유 상태에 기록
pub fn run(shared: Arc<Mutex<Shared>>) {
    loop {
        let info = query();
        if let Ok(mut s) = shared.lock() {
            s.share_info = info;
        }
        thread::sleep(REFRESH_INTERVAL);
    }
}
