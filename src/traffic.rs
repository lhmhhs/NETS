//! 실시간 네트워크 속도 추산 모듈
//! 1초마다 `netstat -e`로 전체 어댑터의 수신/송신 바이트 누계를 읽고, 증가량으로 Mbps를 계산합니다.
//! 외부 인터넷 트래픽뿐 아니라 같은 PC의 모든 송수신이 포함된 추산값입니다.

use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::Shared;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// 그래프에 보관할 최대 샘플 수 (1초 간격 → 2분)
const MAX_HISTORY: usize = 120;
/// 5초 단위 통신량 구간
const BUCKET_INTERVAL: Duration = Duration::from_secs(5);
/// 5초 구간 기록 보관 수 (1시간)
const MAX_BUCKETS: usize = 720;

/// 5초 구간 통신량 (MB, 전체 어댑터 기준)
#[derive(Clone, Debug)]
pub struct Bucket5 {
    /// 프로그램 시작 후 구간 종료 시점(초)
    pub t: f64,
    pub rx_mb: f64,
    pub tx_mb: f64,
}

#[derive(Clone, Debug)]
pub struct TrafficSample {
    /// 프로그램 시작 후 경과 시간(초)
    pub t: f64,
    /// 수신 속도 (Mbps)
    pub rx_mbps: f64,
    /// 송신 속도 (Mbps)
    pub tx_mbps: f64,
}

fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// `netstat -e`의 Bytes 줄에서 (수신 바이트, 송신 바이트) 읽기
/// 줄 이름은 언어마다 다르므로 "마지막 두 토큰이 숫자인 첫 줄"로 찾습니다.
fn read_bytes() -> Option<(u64, u64)> {
    let out = hidden(Command::new("netstat").arg("-e")).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    for line in text.lines() {
        let toks: Vec<&str> = line.split_whitespace().collect();
        if toks.len() >= 3 {
            let rx = toks[toks.len() - 2].parse::<u64>();
            let tx = toks[toks.len() - 1].parse::<u64>();
            if let (Ok(r), Ok(t)) = (rx, tx) {
                return Some((r, t));
            }
        }
    }
    None
}

/// 트래픽 스레드: 1초마다 누계 바이트를 읽어 속도를 계산
pub fn run(shared: Arc<Mutex<Shared>>) {
    let mut prev: Option<((u64, u64), Instant)> = None;
    // 5초 구간 누적 바이트 (수신, 송신)
    let mut acc: (u64, u64) = (0, 0);
    let mut bucket_start = Instant::now();
    loop {
        thread::sleep(Duration::from_secs(1));
        let now = Instant::now();
        let Some((rx, tx)) = read_bytes() else {
            continue;
        };

        if let Some(((prx, ptx), pt)) = prev {
            let dt = now.duration_since(pt).as_secs_f64().max(1e-3);
            let rx_mbps = rx.saturating_sub(prx) as f64 * 8.0 / 1_000_000.0 / dt;
            let tx_mbps = tx.saturating_sub(ptx) as f64 * 8.0 / 1_000_000.0 / dt;
            acc.0 += rx.saturating_sub(prx);
            acc.1 += tx.saturating_sub(ptx);

            // 5초가 지나면 구간 합계를 기록하고 새로 시작
            let finished = if bucket_start.elapsed() >= BUCKET_INTERVAL {
                let b = acc;
                acc = (0, 0);
                bucket_start = now;
                Some(b)
            } else {
                None
            };

            if let Ok(mut s) = shared.lock() {
                let t = s.started.elapsed().as_secs_f64();
                s.traffic.push_back(TrafficSample { t, rx_mbps, tx_mbps });
                while s.traffic.len() > MAX_HISTORY {
                    s.traffic.pop_front();
                }
                if let Some((brx, btx)) = finished {
                    s.traffic5.push_back(Bucket5 {
                        t,
                        rx_mb: brx as f64 / 1_000_000.0,
                        tx_mb: btx as f64 / 1_000_000.0,
                    });
                    while s.traffic5.len() > MAX_BUCKETS {
                        s.traffic5.pop_front();
                    }
                }
            }
        }
        prev = Some(((rx, tx), now));
    }
}
