//! MAC 주소 앞 6자리(OUI)로 제조사를 추정합니다.
//! 전체 IEEE 목록이 아니라 자주 보이는 제조사만 담은 소형 표입니다.

/// (OUI 6자리 HEX, 제조사/분류)
const TABLE: &[(&str, &str)] = &[
    ("000C29", "VMware (VM)"),
    ("005056", "VMware (VM)"),
    ("080027", "VirtualBox (VM)"),
    ("00155D", "Microsoft Hyper-V (VM)"),
    ("001C42", "Parallels (VM)"),
    ("B827EB", "Raspberry Pi"),
    ("DCA632", "Raspberry Pi"),
    ("E45F01", "Raspberry Pi"),
    ("000393", "Apple"),
    ("001CB3", "Apple"),
    ("28CFE9", "Apple"),
    ("3C5AB4", "Google"),
    ("F4F5D8", "Google"),
    ("00E04C", "Realtek"),
    ("001B21", "Intel"),
    ("50C7BF", "TP-Link"),
    ("98DAC4", "TP-Link"),
    ("245A4C", "Ubiquiti"),
    ("0418D6", "Ubiquiti"),
    ("A00460", "Netgear"),
    ("20E52A", "Netgear"),
    ("2C56DC", "ASUS"),
    ("001A92", "ASUS"),
    ("0050F2", "Microsoft"),
];

/// MAC 주소("AA-BB-CC-DD-EE-FF" 또는 "AA:BB:...")에서 제조사 이름을 찾습니다.
pub fn lookup(mac: &str) -> Option<&'static str> {
    let hex: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(12)
        .collect::<String>()
        .to_uppercase();
    if hex.len() < 6 {
        return None;
    }
    if let Some((_, name)) = TABLE.iter().find(|(p, _)| *p == &hex[..6]) {
        return Some(name);
    }
    // 첫 바이트의 2번째 비트(locally administered)가 켜져 있으면 랜덤 MAC
    let first = u8::from_str_radix(&hex[..2], 16).ok()?;
    if first & 0x02 != 0 {
        return Some("Random MAC (privacy)");
    }
    None
}
