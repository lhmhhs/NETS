// 프로그램을 관리자 권한으로 실행하도록 매니페스트를 링크합니다 (MSVC 전용).
// 연결별 누적 바이트(TCP EStats) 조회에 관리자 권한이 필요합니다.
fn main() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
    if target_os == "windows" && target_env == "msvc" {
        println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bins=/MANIFESTUAC:level='requireAdministrator' uiAccess='false'"
        );
    }
    println!("cargo:rerun-if-changed=build.rs");

    // 실행 파일 아이콘(퍼즐 + 네트워크)과 버전 정보 (Windows 탐색기/작업 표시줄용)
    if target_os == "windows" {
        println!("cargo:rerun-if-changed=assets/nets.ico");
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/nets.ico");
        res.set("ProductName", "MRPUZZLE-NETS");
        res.set("FileDescription", "MRPUZZLE-NETS Network Monitor");
        res.set("CompanyName", "MRPUZZLE");
        res.set("LegalCopyright", "(C) MRPUZZLE");
        res.set("OriginalFilename", "nets.exe");
        res.compile().expect("아이콘 리소스 컴파일 실패 (rc.exe 필요)");
    }
}
