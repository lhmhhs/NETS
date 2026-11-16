//! 화면 배치 설정 저장/불러오기
//! 설정 파일(nets_settings.ini)은 실행 파일과 같은 폴더에 저장됩니다.
//! 형식: 한 줄에 "키=값" (값은 쉼표로 구분한 숫자)

use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Layout {
    /// 창 크기 (너비, 높이)
    pub win: Option<[f32; 2]>,
    /// 창 위치 (x, y)
    pub pos: Option<[f32; 2]>,
    /// 왼쪽 패널 너비
    pub left: Option<f32>,
    /// 오른쪽 패널 너비
    pub right: Option<f32>,
    /// 왼쪽 위(네트워크 설정) 패널 높이
    pub net_h: Option<f32>,
    /// 왼쪽 아래(공유 폴더) 패널 높이
    pub share_h: Option<f32>,
    /// 언어 번호 (i18n::LANGS 의 순서)
    pub lang: Option<u8>,
}

fn path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("nets_settings.ini")))
        .unwrap_or_else(|| PathBuf::from("nets_settings.ini"))
}

pub fn load() -> Layout {
    let mut l = Layout::default();
    let Ok(text) = fs::read_to_string(path()) else {
        return l;
    };
    for line in text.lines() {
        let Some((key, val)) = line.split_once('=') else {
            continue;
        };
        let nums: Vec<f32> = val
            .split(',')
            .filter_map(|x| x.trim().parse::<f32>().ok())
            .collect();
        match (key.trim(), nums.as_slice()) {
            ("win", [w, h]) => l.win = Some([*w, *h]),
            ("pos", [x, y]) => l.pos = Some([*x, *y]),
            ("left", [v]) => l.left = Some(*v),
            ("right", [v]) => l.right = Some(*v),
            ("net_h", [v]) => l.net_h = Some(*v),
            ("share_h", [v]) => l.share_h = Some(*v),
            ("lang", [v]) => l.lang = Some(*v as u8),
            _ => {}
        }
    }
    l
}

pub fn save(l: &Layout) {
    let mut s = String::new();
    if let Some([w, h]) = l.win {
        s.push_str(&format!("win={w},{h}\n"));
    }
    if let Some([x, y]) = l.pos {
        s.push_str(&format!("pos={x},{y}\n"));
    }
    if let Some(v) = l.left {
        s.push_str(&format!("left={v}\n"));
    }
    if let Some(v) = l.right {
        s.push_str(&format!("right={v}\n"));
    }
    if let Some(v) = l.net_h {
        s.push_str(&format!("net_h={v}\n"));
    }
    if let Some(v) = l.share_h {
        s.push_str(&format!("share_h={v}\n"));
    }
    if let Some(v) = l.lang {
        s.push_str(&format!("lang={v}\n"));
    }
    let _ = fs::write(path(), s);
}
