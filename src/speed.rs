//! 외부 연결 속도 측정 모듈
//! 5초마다 외부 서버에서 최대 100KB를 내려받아 속도(KB/s, Mbps)와 응답시간을 측정합니다.
//! 기본 측정(Cloudflare) 외에 지역별 서버(REGIONS)도 같은 방식으로 측정합니다.

use std::io::Read;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::Shared;

/// 측정 크기: 100KB (102400 bytes)
const TEST_BYTES: usize = 100 * 1024;
/// 기본 측정: Cloudflare 속도 측정 엔드포인트 (정확히 100KB 응답)
const DEFAULT_URL: &str = "https://speed.cloudflare.com/__down?bytes=102400";
/// 기본 측정 이름
/// 기본 측정 이름 (번역 키)
pub const DEFAULT_NAME: &str = "base_name";
/// 측정 주기
pub const INTERVAL: Duration = Duration::from_secs(5);
/// 그래프/통계에 보관할 최대 샘플 수 (5초 주기 기준 1시간)
const MAX_HISTORY: usize = 720;

/// 지역별 측정 서버.
/// - ranged = true : 큰 파일에 Range 헤더를 보내 앞 100KB만 받습니다 (Vultr 테스트 파일).
/// - ranged = false: 웹페이지를 받아 최대 100KB까지 읽습니다 (daum.net, baidu.com).
///   페이지가 100KB보다 작으면 실제로 받은 만큼만 측정합니다.
/// url 이 빈 문자열("")이면 측정하지 않고 "URL 미설정"으로 표시합니다.
pub struct Region {
    pub name: &'static str,
    pub url: &'static str,
    pub ranged: bool,
}

pub const REGIONS: &[Region] = &[
    Region {
        name: "region_us",
        url: "https://lax-ca-us-ping.vultr.com/vultr.com.100MB.bin",
        ranged: true,
    },
    Region {
        name: "region_jp",
        url: "https://hnd-jp-ping.vultr.com/vultr.com.100MB.bin",
        ranged: true,
    },
    Region {
        name: "region_sg",
        url: "https://sgp-ping.vultr.com/vultr.com.100MB.bin",
        ranged: true,
    },
    Region {
        name: "region_kr",
        url: "https://www.daum.net/",
        ranged: false,
    },
    Region {
        name: "region_cn",
        url: "https://www.baidu.com/",
        ranged: false,
    },
];

#[derive(Clone, Debug)]
pub struct Sample {
    /// 프로그램 시작 후 경과 시간(초)
    pub t: f64,
    /// 다운로드 속도 (KB/s)
    pub kb_s: f64,
    /// 다운로드 속도 (Mbps)
    pub mbps: f64,
    /// 응답 시간 (헤더 수신까지, ms)
    pub latency_ms: f64,
    /// 전체 소요 시간 (초)
    pub secs: f64,
}

/// 지역 측정 상태 (화면에서 언어에 맞게 표시)
#[derive(Clone, Debug)]
pub enum RegionStatus {
    /// 측정 완료 (받은 KB)
    Done(usize),
    /// URL 미설정
    Unset,
    /// 실패 (원인 문자열)
    Failed(String),
}

/// 지역 측정 결과 (가장 최근 주기)
#[derive(Clone, Debug)]
pub struct RegionResult {
    /// 번역 키 (region_us 등)
    pub name: &'static str,
    pub kb_s: Option<f64>,
    pub mbps: Option<f64>,
    pub latency_ms: Option<f64>,
    pub status: RegionStatus,
}

/// 그래프용 한 주기의 지역별 속도 (기본 측정 포함, 순서 고정)
#[derive(Clone, Debug)]
pub struct RegionPoint {
    pub t: f64,
    pub values: Vec<(&'static str, Option<f64>)>,
}

/// 다운로드 한 번 수행 → (헤더 응답시간 ms, 전체 소요 초, 받은 바이트)
/// `ranged` 가 true 면 Range 헤더로 앞 100KB만 요청합니다.
/// 어떤 경우든 최대 TEST_BYTES 까지만 읽습니다.
fn measure(url: &str, ranged: bool) -> Result<(f64, f64, usize), String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();

    let mut req = agent
        .get(url)
        .set("Cache-Control", "no-cache")
        .set("Accept-Encoding", "identity")
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36",
        );
    if ranged {
        req = req.set("Range", &format!("bytes=0-{}", TEST_BYTES - 1));
    }

    let start = Instant::now();
    let resp = req.call().map_err(|e| e.to_string())?;
    let latency_ms = start.elapsed().as_secs_f64() * 1000.0;

    let mut reader = resp.into_reader();
    let mut buf = [0u8; 16 * 1024];
    let mut total = 0usize;
    while total < TEST_BYTES {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        total += n.min(TEST_BYTES - total);
    }
    Ok((latency_ms, start.elapsed().as_secs_f64(), total))
}

/// 바이트/시간 → (KB/s, Mbps)
fn rates(bytes: usize, secs: f64) -> (f64, f64) {
    let secs = secs.max(1e-6);
    (
        bytes as f64 / 1024.0 / secs,
        bytes as f64 * 8.0 / 1_000_000.0 / secs,
    )
}

/// 모든 지역을 순서대로 측정
fn measure_regions() -> Vec<RegionResult> {
    REGIONS
        .iter()
        .map(|r| {
            if r.url.is_empty() {
                return RegionResult {
                    name: r.name,
                    kb_s: None,
                    mbps: None,
                    latency_ms: None,
                    status: RegionStatus::Unset,
                };
            }
            match measure(r.url, r.ranged) {
                Ok((latency, secs, bytes)) => {
                    let (kb_s, mbps) = rates(bytes, secs);
                    RegionResult {
                        name: r.name,
                        kb_s: Some(kb_s),
                        mbps: Some(mbps),
                        latency_ms: Some(latency),
                        status: RegionStatus::Done(bytes / 1024),
                    }
                }
                Err(e) => RegionResult {
                    name: r.name,
                    kb_s: None,
                    mbps: None,
                    latency_ms: None,
                    status: RegionStatus::Failed(e),
                },
            }
        })
        .collect()
}

/// 측정 스레드: 5초마다 기본 측정과 지역 측정을 수행하고 공유 상태에 기록
pub fn run(shared: Arc<Mutex<Shared>>) {
    loop {
        let cycle_start = Instant::now();

        if let Ok(mut s) = shared.lock() {
            s.testing = true;
            s.next_test = None;
        }

        let result = measure(DEFAULT_URL, false);
        let regions = measure_regions();

        // 그래프용 지역별 값 (기본 측정 + 지역 순서)
        let mut values: Vec<(&'static str, Option<f64>)> = vec![(
            DEFAULT_NAME,
            result
                .as_ref()
                .ok()
                .map(|&(_, secs, bytes)| rates(bytes, secs).0),
        )];
        values.extend(regions.iter().map(|r| (r.name, r.kb_s)));

        if let Ok(mut s) = shared.lock() {
            s.testing = false;
            s.regions = regions;
            match result {
                Ok((latency_ms, secs, bytes)) => {
                    let (kb_s, mbps) = rates(bytes, secs);
                    let t = s.started.elapsed().as_secs_f64();
                    s.samples.push_back(Sample {
                        t,
                        kb_s,
                        mbps,
                        latency_ms,
                        secs,
                    });
                    while s.samples.len() > MAX_HISTORY {
                        s.samples.pop_front();
                    }
                    s.last_error = None;
                }
                Err(e) => s.last_error = Some(e),
            }
            let t = s.started.elapsed().as_secs_f64();
            s.region_history.push_back(RegionPoint { t, values });
            while s.region_history.len() > MAX_HISTORY {
                s.region_history.pop_front();
            }
        }

        let wait = INTERVAL.saturating_sub(cycle_start.elapsed());
        if let Ok(mut s) = shared.lock() {
            s.next_test = Some(Instant::now() + wait);
        }
        thread::sleep(wait);
    }
}
