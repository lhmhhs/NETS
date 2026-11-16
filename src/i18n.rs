//! 다국어 지원 (한국어, English, Français, Deutsch, 日本語, 中文, Русский)
//! 현재 언어는 전역 값으로 보관하며, t("키")로 문자열을 가져옵니다.
//! 키가 없으면 영어, 그것도 없으면 키 자체를 돌려줍니다.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Ko,
    En,
    Fr,
    De,
    Ja,
    Zh,
    Ru,
    Es,
}

/// 선택 목록 표시 순서
pub const LANGS: [Lang; 8] = [
    Lang::Ko,
    Lang::En,
    Lang::Fr,
    Lang::De,
    Lang::Es,
    Lang::Ja,
    Lang::Zh,
    Lang::Ru,
];

impl Lang {
    /// 언어 선택 목록에 보이는 이름 (각 언어의 표기)
    pub fn label(self) -> &'static str {
        match self {
            Lang::Ko => "한국어",
            Lang::En => "English",
            Lang::Fr => "Français",
            Lang::De => "Deutsch",
            Lang::Es => "Español",
            Lang::Ja => "日本語",
            Lang::Zh => "中文",
            Lang::Ru => "Русский",
        }
    }

    /// 설정 파일에 저장하는 고정 번호 (표시 순서와 무관)
    pub fn index(self) -> usize {
        match self {
            Lang::Ko => 0,
            Lang::En => 1,
            Lang::Fr => 2,
            Lang::De => 3,
            Lang::Ja => 4,
            Lang::Zh => 5,
            Lang::Ru => 6,
            Lang::Es => 7,
        }
    }

    pub fn from_index(i: u8) -> Lang {
        match i {
            1 => Lang::En,
            2 => Lang::Fr,
            3 => Lang::De,
            4 => Lang::Ja,
            5 => Lang::Zh,
            6 => Lang::Ru,
            7 => Lang::Es,
            _ => Lang::Ko,
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(l: Lang) {
    CURRENT.store(l.index() as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    Lang::from_index(CURRENT.load(Ordering::Relaxed))
}

/// 현재 언어의 문자열
pub fn t(key: &str) -> String {
    let lang = current();
    if lang == Lang::Es {
        if let Some((_, v)) = ES_TABLE.iter().find(|(k, _)| *k == key) {
            return v.to_string();
        }
    }
    // 스페인어에 없는 키는 영어 문자열 사용
    let idx = if lang == Lang::Es { 1 } else { lang.index() };
    match TABLE.iter().find(|(k, _)| *k == key) {
        Some((_, v)) => {
            let s = v[idx];
            if !s.is_empty() {
                s.to_string()
            } else {
                v[1].to_string()
            }
        }
        None => key.to_string(),
    }
}

/// {name} 자리를 값으로 바꾼 문자열
pub fn tf(key: &str, args: &[(&str, String)]) -> String {
    let mut s = t(key);
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}

/// 스페인어 문자열 (키 → 문장). 여기 없는 키는 영어로 표시됩니다.
const ES_TABLE: &[(&str, &str)] = &[
    ("app_title", "MRPUZZLE-NETS - Monitor de red"),
    ("net_monitor", "Monitor de red"),
    ("language", "Idioma"),
    ("my_ip", "Mi IP: {ip}"),
    ("scan_now", "Escanear ahora"),
    ("help", "Ayuda"),
    ("scanning", "Escaneando..."),
    ("found_n", "{n} dispositivos encontrados"),
    ("no_network", "Sin conexión de red"),
    ("last_scan_ago", "Último escaneo hace {s} s"),
    ("not_scanned", "Sin escanear"),
    ("auto_refresh_60", "· La lista de dispositivos se actualiza cada 60 s"),
    ("lan", "LAN"),
    ("card_net", "Mi configuración de red"),
    ("net_loading", "Cargando información de red o sin conexión."),
    ("conn_type", "Tipo de conexión"),
    ("wired", "Cableada (Ethernet)"),
    ("adapter", "Adaptador"),
    ("description", "Descripción"),
    ("ipv4_all", "Direcciones IPv4 (todas)"),
    ("subnet", "Máscara de subred"),
    ("gateway", "Puerta de enlace predeterminada"),
    ("dns", "Servidores DNS"),
    ("ip_assign", "Asignación de IP"),
    ("dhcp_auto", "Automática (DHCP)"),
    ("dhcp_manual", "Manual (estática)"),
    ("mac", "Dirección MAC"),
    ("link_speed", "Velocidad de enlace"),
    ("signal", "Intensidad de señal"),
    ("rx_link_speed", "Velocidad de recepción del enlace"),
    ("current_dl", "Descarga actual"),
    ("waiting", "Esperando medición"),
    ("net_refresh_note", "La configuración se actualiza cada 30 s."),
    ("card_conn", "Puertos externos · tráfico de 5 s"),
    ("recent5", "Tráfico de los últimos 5 s"),
    ("rx_label", "↓ Recibido"),
    ("tx_label", "↑ Enviado"),
    ("time", "Hora"),
    ("rx_mb", "Recibido MB"),
    ("tx_mb", "Enviado MB"),
    ("conn_count", "{n} conexiones externas"),
    ("no_conns", "No hay puertos conectados al exterior."),
    ("remote_ip", "IP remota"),
    ("country", "País"),
    ("port", "Puerto"),
    ("process", "Proceso"),
    ("count", "Nº"),
    ("unknown", "(desconocido)"),
    ("port_totals_title", "Total acumulado por puerto (desde el inicio)"),
    ("no_totals", "Aún no hay tráfico registrado."),
    ("total_rx", "Total recibido"),
    ("total_tx", "Total enviado"),
    ("service", "Servicio"),
    ("conn_footnote", "El tráfico de 5 s corresponde a todos los adaptadores de este PC. Los totales por puerto usan TCP EStats e incluyen solo conexiones IPv4."),
    ("note_enable_fail", "No se pudo activar la estadística de conexión (código {n}). Compruebe que NETS se ejecuta como administrador."),
    ("note_read_fail", "No se pudo leer la estadística de conexión (código {n})"),
    ("card_share", "Mis carpetas compartidas / conexiones"),
    ("shares_count", "{n} carpetas compartidas"),
    ("no_shares", "No hay carpetas compartidas."),
    ("peers_count", "{n} conectados"),
    ("no_peers", "No hay usuarios conectados."),
    ("no_name", "(sin nombre)"),
    ("share_footnote", "Las conexiones se basan en SMB (445) y se actualizan cada 15 s."),
    ("card_devices", "Dispositivos de red cercanos"),
    ("devices_summary", "{n} encontrados · {m} responden"),
    ("me", "Este PC"),
    ("gateway_kind", "Puerta de enlace (router)"),
    ("device", "Dispositivo"),
    ("online", "En línea"),
    ("arp_only", "Solo caché ARP"),
    ("name_label", "Nombre: {name}"),
    ("vendor_label", "Fabricante: {vendor}"),
    ("vendor_unknown", "Fabricante desconocido"),
    ("mac_label", "MAC: {mac}"),
    ("card_speed", "Velocidad de red"),
    ("live_title", "Estimación en vivo (1 s · todo el tráfico de este PC)"),
    ("dl_title", "Velocidad de descarga externa (100 KB · cada 5 s)"),
    ("measuring", "Midiendo..."),
    ("next_in", "Próxima medición en {n} s"),
    ("error_prefix", "Error: {err}"),
    ("avg", "Media"),
    ("max", "Máx."),
    ("min", "Mín."),
    ("latency", "Tiempo de respuesta"),
    ("sample_count", "Nº de mediciones"),
    ("recent_records", "Registros recientes"),
    ("hdr_resp", "Resp."),
    ("elapsed", "Tiempo transcurrido (s)"),
    ("dl_legend", "Descarga KB/s"),
    ("rx_legend", "Recibido Mbps"),
    ("tx_legend", "Enviado Mbps"),
    ("region_graph_title", "Velocidad de descarga por región (KB/s)"),
    ("region_table_title", "Velocidad de descarga por región (100 KB · cada 5 s)"),
    ("region", "Región"),
    ("status", "Estado"),
    ("status_done", "Completado ({n} KB)"),
    ("status_unset", "URL no configurada"),
    ("base_name", "Predeterminada (Cloudflare)"),
    ("region_us", "EE. UU. (LA)"),
    ("region_jp", "Japón (Tokio)"),
    ("region_sg", "Singapur"),
    ("region_kr", "Corea (daum.net)"),
    ("region_cn", "China (Baidu baidu.com)"),
    ("help_title", "Ayuda · MRPUZZLE-NETS"),
    ("help_heading", "MRPUZZLE-NETS - Monitor de red"),
    ("help_version", "Versión {v}"),
    ("help_author", "Autor: MRPUZZLE (C)"),
    ("help_free", "Uso gratuito: cualquiera puede usar este programa sin coste."),
    ("help_screens", "Distribución de la pantalla"),
    ("help_s1", "· Arriba a la izquierda: mi configuración de red (IP, subred, puerta de enlace, DNS, MAC, velocidad de enlace, etc.)"),
    ("help_s2", "· Centro izquierda: puertos externos (ordenados por país), tráfico de 5 s, totales por puerto"),
    ("help_s3", "· Abajo a la izquierda: mis carpetas compartidas y las conexiones SMB (IP/nombre)"),
    ("help_s4", "· Centro: dispositivos de red cercanos (IP, nombre, fabricante estimado, MAC, tiempo de respuesta)"),
    ("help_s5", "· Derecha: estimación de envío/recepción (Mbps), velocidad de descarga externa (100 KB, cada 5 s), tabla y gráfico por región"),
    ("help_s6", "· Arrastre los bordes de los paneles para cambiar su tamaño. La disposición, la posición y el tamaño de la ventana se guardan en nets_settings.ini y se restauran al iniciar."),
    ("help_s7", "· Escanear ahora: busca dispositivos cercanos de inmediato. La lista se actualiza cada 60 s."),
    ("help_s8", "· Pase el ratón sobre el gráfico para ver la velocidad de cada región en ese instante."),
    ("help_runtime", "Requisitos"),
    ("help_r1", "· Windows 10 / 11 (64 bits)"),
    ("help_r2", "· Ejecutar como administrador: necesario para los totales por puerto (estadísticas TCP). Al iniciar aparece una ventana UAC."),
    ("help_r3", "· Conexión a Internet: necesaria para las pruebas externas (Cloudflare, Vultr, daum.net, baidu.com) y la consulta de país (ipinfo.io)."),
    ("help_r4", "· Herramientas del sistema: ping, arp, netstat, PowerShell (incluidas en Windows)"),
    ("help_license", "Licencias y componentes"),
    ("lic_rust", "· Lenguaje Rust y biblioteca estándar: MIT OR Apache-2.0"),
    ("lic_egui", "· eframe / egui (GUI): MIT OR Apache-2.0"),
    ("lic_plot", "· egui_plot (gráficos): MIT OR Apache-2.0"),
    ("lic_ureq", "· ureq (solicitudes HTTP): MIT OR Apache-2.0"),
    ("lic_font", "· Fuente Malgun Gothic: fuente de Microsoft Windows (se aplican las condiciones de licencia de Windows)"),
    ("lic_services", "· Servicios externos: prueba de Cloudflare, archivos de prueba de Vultr, daum.net, baidu.com, ipinfo.io (sujetos a las condiciones de cada servicio)"),
    ("help_free_modify", "Modificación libre: también puede modificar y mejorar el código libremente."),
    ("help_attrib", "Atribución obligatoria: al usar, modificar o redistribuir, debe indicar que se basa en «MRPUZZLE(C) NETS»."),
    ("help_license_name", "Licencia: licencia libre con atribución obligatoria (similar a Creative Commons CC BY 4.0, MIT)"),
    ("lic_note", "Estas licencias pertenecen a los autores originales de cada componente y se aplican sus condiciones. Los servicios externos pueden interrumpirse o cambiar sus valores según decida el proveedor."),
];

/// 키 → [한국어, English, Français, Deutsch, 日本語, 中文, Русский]
const TABLE: &[(&str, [&str; 7])] = &[
    // ----- 공통 / 상단 -----
    ("app_title", ["MRPUZZLE-NETS - 네트워크 모니터", "MRPUZZLE-NETS - Network Monitor", "MRPUZZLE-NETS - Moniteur réseau", "MRPUZZLE-NETS - Netzwerkmonitor", "MRPUZZLE-NETS - ネットワークモニター", "MRPUZZLE-NETS - 网络监控", "MRPUZZLE-NETS - Монитор сети"]),
    ("net_monitor", ["네트워크 모니터", "Network Monitor", "Moniteur réseau", "Netzwerkmonitor", "ネットワークモニター", "网络监控", "Монитор сети"]),
    ("language", ["언어", "Language", "Langue", "Sprache", "言語", "语言", "Язык"]),
    ("my_ip", ["내 IP: {ip}", "My IP: {ip}", "Mon IP : {ip}", "Meine IP: {ip}", "自分のIP: {ip}", "我的IP: {ip}", "Мой IP: {ip}"]),
    ("scan_now", ["지금 스캔", "Scan now", "Scanner", "Jetzt scannen", "今すぐスキャン", "立即扫描", "Сканировать"]),
    ("help", ["도움말", "Help", "Aide", "Hilfe", "ヘルプ", "帮助", "Справка"]),
    ("scanning", ["스캔 중...", "Scanning...", "Analyse en cours...", "Suche läuft...", "スキャン中...", "扫描中...", "Сканирование..."]),
    ("found_n", ["{n}개 장치 발견", "{n} devices found", "{n} appareils trouvés", "{n} Geräte gefunden", "{n} 台のデバイスを検出", "发现 {n} 台设备", "Найдено устройств: {n}"]),
    ("no_network", ["네트워크 연결이 없습니다", "No network connection", "Pas de connexion réseau", "Keine Netzwerkverbindung", "ネットワーク接続がありません", "没有网络连接", "Нет сетевого подключения"]),
    ("last_scan_ago", ["마지막 스캔 {s}초 전", "Last scan {s} s ago", "Dernier scan il y a {s} s", "Letzter Scan vor {s} s", "最終スキャン {s} 秒前", "上次扫描 {s} 秒前", "Последнее сканирование {s} с назад"]),
    ("not_scanned", ["스캔 전", "Not scanned yet", "Pas encore analysé", "Noch nicht gescannt", "未スキャン", "尚未扫描", "Ещё не сканировалось"]),
    ("auto_refresh_60", ["· 장치 목록은 60초마다 자동 갱신", "· Device list refreshes every 60 s", "· Liste actualisée toutes les 60 s", "· Geräteliste alle 60 s aktualisiert", "· デバイス一覧は60秒ごとに自動更新", "· 设备列表每60秒自动刷新", "· Список обновляется каждые 60 с"]),
    ("lan", ["LAN", "LAN", "LAN", "LAN", "LAN", "LAN", "LAN"]),
    // ----- 내 네트워크 설정 -----
    ("card_net", ["내 네트워크 설정", "My network settings", "Paramètres réseau", "Netzwerkeinstellungen", "ネットワーク設定", "我的网络设置", "Настройки сети"]),
    ("net_loading", ["네트워크 정보를 불러오는 중이거나 연결이 없습니다.", "Loading network info or not connected.", "Chargement des infos réseau ou pas de connexion.", "Netzwerkinfos werden geladen oder keine Verbindung.", "ネットワーク情報を読み込み中、または未接続です。", "正在加载网络信息或未连接。", "Загрузка сетевых данных или нет подключения."]),
    ("conn_type", ["연결 종류", "Connection", "Type de connexion", "Verbindungsart", "接続種別", "连接类型", "Тип подключения"]),
    ("wired", ["유선 (이더넷)", "Wired (Ethernet)", "Filaire (Ethernet)", "Kabel (Ethernet)", "有線 (イーサネット)", "有线 (以太网)", "Проводное (Ethernet)"]),
    ("adapter", ["어댑터", "Adapter", "Adaptateur", "Adapter", "アダプター", "适配器", "Адаптер"]),
    ("description", ["설명", "Description", "Description", "Beschreibung", "説明", "说明", "Описание"]),
    ("ipv4_all", ["IPv4 주소 (전체)", "IPv4 addresses (all)", "Adresses IPv4 (toutes)", "IPv4-Adressen (alle)", "IPv4アドレス (全件)", "IPv4 地址 (全部)", "IPv4-адреса (все)"]),
    ("subnet", ["서브넷 마스크", "Subnet mask", "Masque de sous-réseau", "Subnetzmaske", "サブネットマスク", "子网掩码", "Маска подсети"]),
    ("gateway", ["기본 게이트웨이", "Default gateway", "Passerelle par défaut", "Standardgateway", "デフォルトゲートウェイ", "默认网关", "Шлюз по умолчанию"]),
    ("dns", ["DNS 서버", "DNS servers", "Serveurs DNS", "DNS-Server", "DNSサーバー", "DNS 服务器", "DNS-серверы"]),
    ("ip_assign", ["IP 할당", "IP assignment", "Attribution IP", "IP-Zuweisung", "IP割り当て", "IP 分配", "Назначение IP"]),
    ("dhcp_auto", ["자동 (DHCP)", "Automatic (DHCP)", "Automatique (DHCP)", "Automatisch (DHCP)", "自動 (DHCP)", "自动 (DHCP)", "Автоматически (DHCP)"]),
    ("dhcp_manual", ["수동 (고정)", "Manual (static)", "Manuel (statique)", "Manuell (statisch)", "手動 (固定)", "手动 (固定)", "Вручную (статически)"]),
    ("mac", ["MAC 주소", "MAC address", "Adresse MAC", "MAC-Adresse", "MACアドレス", "MAC 地址", "MAC-адрес"]),
    ("link_speed", ["링크 속도", "Link speed", "Vitesse de liaison", "Verbindungsgeschwindigkeit", "リンク速度", "链路速度", "Скорость соединения"]),
    ("signal", ["신호 강도", "Signal strength", "Force du signal", "Signalstärke", "信号強度", "信号强度", "Уровень сигнала"]),
    ("rx_link_speed", ["수신 링크 속도", "Receive link speed", "Vitesse de réception", "Empfangsgeschwindigkeit", "受信リンク速度", "接收链路速度", "Скорость приёма"]),
    ("current_dl", ["현재 다운로드", "Current download", "Téléchargement actuel", "Aktueller Download", "現在のダウンロード", "当前下载", "Текущая загрузка"]),
    ("waiting", ["측정 대기 중", "Waiting for test", "En attente de mesure", "Warte auf Messung", "測定待ち", "等待测量", "Ожидание измерения"]),
    ("net_refresh_note", ["설정 정보는 30초마다 갱신됩니다.", "Settings refresh every 30 s.", "Actualisé toutes les 30 s.", "Aktualisierung alle 30 s.", "設定は30秒ごとに更新されます。", "设置每30秒刷新。", "Обновляется каждые 30 с."]),
    // ----- 외부 연결 / 5초 통신량 / 포트 -----
    ("card_conn", ["외부 연결 포트 · 5초 통신량", "External ports · 5-second traffic", "Ports externes · trafic 5 s", "Externe Ports · Datenverkehr 5 s", "外部ポート · 5秒間の通信量", "外部端口 · 5秒流量", "Внешние порты · трафик за 5 с"]),
    ("recent5", ["최근 5초 통신량", "Traffic in last 5 s", "Trafic des 5 dernières s", "Datenverkehr letzte 5 s", "直近5秒の通信量", "最近5秒流量", "Трафик за последние 5 с"]),
    ("rx_label", ["↓ 수신", "↓ Received", "↓ Reçu", "↓ Empfangen", "↓ 受信", "↓ 接收", "↓ Приём"]),
    ("tx_label", ["↑ 송신", "↑ Sent", "↑ Envoyé", "↑ Gesendet", "↑ 送信", "↑ 发送", "↑ Отправка"]),
    ("time", ["시간", "Time", "Heure", "Zeit", "時刻", "时间", "Время"]),
    ("rx_mb", ["수신 MB", "Received MB", "Reçu Mo", "Empfangen MB", "受信 MB", "接收 MB", "Принято МБ"]),
    ("tx_mb", ["송신 MB", "Sent MB", "Envoyé Mo", "Gesendet MB", "送信 MB", "发送 MB", "Отправлено МБ"]),
    ("conn_count", ["외부 연결 {n}개", "{n} external connections", "{n} connexions externes", "{n} externe Verbindungen", "外部接続 {n} 件", "外部连接 {n} 个", "Внешних подключений: {n}"]),
    ("no_conns", ["외부로 연결된 포트가 없습니다.", "No external connections.", "Aucune connexion externe.", "Keine externen Verbindungen.", "外部への接続はありません。", "没有外部连接。", "Нет внешних подключений."]),
    ("remote_ip", ["원격 IP", "Remote IP", "IP distante", "Remote-IP", "リモートIP", "远程 IP", "Удалённый IP"]),
    ("country", ["국가", "Country", "Pays", "Land", "国", "国家", "Страна"]),
    ("port", ["포트", "Port", "Port", "Port", "ポート", "端口", "Порт"]),
    ("process", ["프로세스", "Process", "Processus", "Prozess", "プロセス", "进程", "Процесс"]),
    ("count", ["수", "Count", "Nb", "Anz.", "数", "数量", "Кол-во"]),
    ("unknown", ["(알 수 없음)", "(unknown)", "(inconnu)", "(unbekannt)", "(不明)", "(未知)", "(неизвестно)"]),
    ("port_totals_title", ["포트별 누적 통신량 (NETS 실행 이후)", "Cumulative traffic per port (since start)", "Trafic cumulé par port (depuis le lancement)", "Gesamtverkehr pro Port (seit Start)", "ポート別累計通信量 (起動後)", "各端口累计流量 (启动后)", "Суммарный трафик по портам (с запуска)"]),
    ("no_totals", ["집계된 통신량이 아직 없습니다.", "No traffic recorded yet.", "Aucun trafic enregistré.", "Noch kein Datenverkehr erfasst.", "まだ集計された通信量はありません。", "尚无流量统计。", "Трафик ещё не учтён."]),
    ("total_rx", ["누적 수신", "Total received", "Total reçu", "Gesamt empfangen", "累計受信", "累计接收", "Всего принято"]),
    ("total_tx", ["누적 송신", "Total sent", "Total envoyé", "Gesamt gesendet", "累計送信", "累计发送", "Всего отправлено"]),
    ("service", ["서비스", "Service", "Service", "Dienst", "サービス", "服务", "Служба"]),
    ("conn_footnote", ["5초 통신량은 이 PC 전체 어댑터 기준입니다. 포트별 누적은 연결 통계(TCP EStats)로 집계하며, IPv4 연결만 포함합니다.", "5-second traffic covers all adapters on this PC. Per-port totals use TCP EStats and include IPv4 connections only.", "Le trafic 5 s concerne toutes les interfaces. Les totaux par port utilisent TCP EStats (IPv4 uniquement).", "Der 5-s-Verkehr umfasst alle Adapter. Port-Summen über TCP EStats, nur IPv4.", "5秒の通信量はこのPCの全アダプター基準です。ポート別累計はTCP EStatsで集計し、IPv4のみ対象です。", "5秒流量为本机所有网卡合计。端口累计基于 TCP EStats，仅含 IPv4 连接。", "Трафик за 5 с — по всем адаптерам ПК. Суммы по портам — через TCP EStats, только IPv4."]),
    ("note_enable_fail", ["연결 통계 활성화 실패 (코드 {n}). 관리자 권한으로 실행했는지 확인하세요.", "Failed to enable connection stats (code {n}). Check that NETS runs as administrator.", "Échec d'activation des stats (code {n}). Vérifiez l'exécution en administrateur.", "Verbindungsstatistik konnte nicht aktiviert werden (Code {n}). Als Administrator starten.", "接続統計の有効化に失敗しました (コード {n})。管理者として実行してください。", "启用连接统计失败（代码 {n}）。请确认以管理员身份运行。", "Не удалось включить статистику соединений (код {n}). Запустите от имени администратора."]),
    ("note_read_fail", ["연결 통계 조회 실패 (코드 {n})", "Failed to read connection stats (code {n})", "Échec de lecture des stats (code {n})", "Verbindungsstatistik nicht lesbar (Code {n})", "接続統計の読み取りに失敗 (コード {n})", "读取连接统计失败（代码 {n}）", "Не удалось прочитать статистику (код {n})"]),
    // ----- 공유 폴더 / 접속 -----
    ("card_share", ["내 공유 폴더 / 접속", "My shared folders / connections", "Mes dossiers partagés / connexions", "Meine Freigaben / Verbindungen", "共有フォルダー / 接続", "共享文件夹 / 连接", "Мои общие папки / подключения"]),
    ("shares_count", ["공유 폴더 {n}개", "{n} shared folders", "{n} dossiers partagés", "{n} freigegebene Ordner", "共有フォルダー {n} 件", "共享文件夹 {n} 个", "Общих папок: {n}"]),
    ("no_shares", ["공유 중인 폴더가 없습니다.", "No shared folders.", "Aucun dossier partagé.", "Keine Ordner freigegeben.", "共有中のフォルダーはありません。", "没有共享文件夹。", "Нет общих папок."]),
    ("peers_count", ["접속 {n}명", "{n} connected", "{n} connecté(s)", "{n} verbunden", "接続 {n} 人", "已连接 {n} 人", "Подключено: {n}"]),
    ("no_peers", ["현재 접속 중인 사용자가 없습니다.", "No one is connected.", "Personne n'est connecté.", "Niemand ist verbunden.", "現在接続中のユーザーはいません。", "当前没有连接用户。", "Сейчас никто не подключён."]),
    ("no_name", ["(이름 없음)", "(no name)", "(sans nom)", "(ohne Namen)", "(名前なし)", "(无名称)", "(без имени)"]),
    ("share_footnote", ["접속은 SMB(445) 연결 기준이며 15초마다 갱신됩니다.", "Connections are based on SMB (445) and refresh every 15 s.", "Connexions SMB (445), actualisées toutes les 15 s.", "Verbindungen über SMB (445), alle 15 s aktualisiert.", "接続は SMB(445) 基準で、15秒ごとに更新されます。", "连接基于 SMB(445)，每15秒刷新。", "Подключения по SMB (445), обновление каждые 15 с."]),
    // ----- 주변 장치 -----
    ("card_devices", ["주변 네트워크 장치", "Nearby network devices", "Appareils réseau proches", "Geräte im Netzwerk", "周辺のネットワーク機器", "周边网络设备", "Устройства рядом в сети"]),
    ("devices_summary", ["총 {n}대 발견 · 응답 {m}대", "{n} found · {m} responding", "{n} trouvés · {m} répondent", "{n} gefunden · {m} antworten", "{n} 台発見 · 応答 {m} 台", "发现 {n} 台 · 响应 {m} 台", "Найдено: {n} · отвечает: {m}"]),
    ("me", ["내 PC", "This PC", "Ce PC", "Dieser PC", "このPC", "本机", "Этот ПК"]),
    ("gateway_kind", ["게이트웨이 (공유기)", "Gateway (router)", "Passerelle (routeur)", "Gateway (Router)", "ゲートウェイ (ルーター)", "网关 (路由器)", "Шлюз (роутер)"]),
    ("device", ["장치", "Device", "Appareil", "Gerät", "デバイス", "设备", "Устройство"]),
    ("online", ["온라인", "Online", "En ligne", "Online", "オンライン", "在线", "В сети"]),
    ("arp_only", ["ARP 캐시만", "ARP cache only", "Cache ARP seulement", "Nur ARP-Cache", "ARPキャッシュのみ", "仅 ARP 缓存", "Только кэш ARP"]),
    ("name_label", ["이름: {name}", "Name: {name}", "Nom : {name}", "Name: {name}", "名前: {name}", "名称: {name}", "Имя: {name}"]),
    ("vendor_label", ["제조사: {vendor}", "Vendor: {vendor}", "Fabricant : {vendor}", "Hersteller: {vendor}", "メーカー: {vendor}", "厂商: {vendor}", "Производитель: {vendor}"]),
    ("vendor_unknown", ["제조사 미확인", "Vendor unknown", "Fabricant inconnu", "Hersteller unbekannt", "メーカー不明", "厂商未知", "Производитель неизвестен"]),
    ("mac_label", ["MAC: {mac}", "MAC: {mac}", "MAC : {mac}", "MAC: {mac}", "MAC: {mac}", "MAC: {mac}", "MAC: {mac}"]),
    // ----- 속도 -----
    ("card_speed", ["네트워크 속도", "Network speed", "Vitesse réseau", "Netzwerkgeschwindigkeit", "ネットワーク速度", "网络速度", "Скорость сети"]),
    ("live_title", ["실시간 추산 (1초 단위 · 이 PC 전체 송수신 기준)", "Live estimate (1 s · all traffic on this PC)", "Estimation en direct (1 s · tout le trafic du PC)", "Live-Schätzung (1 s · gesamter Verkehr dieses PCs)", "リアルタイム推定 (1秒単位 · このPC全体)", "实时估算（1秒 · 本机全部收发）", "Оценка в реальном времени (1 с · весь трафик ПК)"]),
    ("dl_title", ["외부 다운로드 속도 (100KB · 5초마다 측정)", "External download speed (100 KB · every 5 s)", "Vitesse de téléchargement (100 Ko · toutes les 5 s)", "Externe Download-Geschwindigkeit (100 KB · alle 5 s)", "外部ダウンロード速度 (100KB · 5秒ごと)", "外部下载速度（100KB · 每5秒）", "Скорость загрузки (100 КБ · каждые 5 с)"]),
    ("measuring", ["측정 중...", "Measuring...", "Mesure...", "Messung...", "測定中...", "测量中...", "Измерение..."]),
    ("next_in", ["다음 측정까지 {n}초", "Next test in {n} s", "Prochaine mesure dans {n} s", "Nächste Messung in {n} s", "次の測定まで {n} 秒", "距下次测量 {n} 秒", "Следующее измерение через {n} с"]),
    ("error_prefix", ["오류: {err}", "Error: {err}", "Erreur : {err}", "Fehler: {err}", "エラー: {err}", "错误: {err}", "Ошибка: {err}"]),
    ("avg", ["평균", "Average", "Moyenne", "Mittel", "平均", "平均", "Среднее"]),
    ("max", ["최고", "Max", "Max", "Max", "最高", "最高", "Макс."]),
    ("min", ["최저", "Min", "Min", "Min", "最低", "最低", "Мин."]),
    ("latency", ["응답시간", "Response time", "Temps de réponse", "Antwortzeit", "応答時間", "响应时间", "Время отклика"]),
    ("sample_count", ["측정 횟수", "Test count", "Nombre de mesures", "Anzahl Messungen", "測定回数", "测量次数", "Количество измерений"]),
    ("recent_records", ["최근 측정 기록", "Recent results", "Derniers résultats", "Letzte Ergebnisse", "最近の測定記録", "最近测量记录", "Последние результаты"]),
    ("hdr_resp", ["응답", "Resp.", "Rép.", "Antw.", "応答", "响应", "Отклик"]),
    ("elapsed", ["경과 시간 (초)", "Elapsed time (s)", "Temps écoulé (s)", "Verstrichene Zeit (s)", "経過時間 (秒)", "经过时间（秒）", "Прошло времени (с)"]),
    ("dl_legend", ["다운로드 KB/s", "Download KB/s", "Téléchargement Ko/s", "Download KB/s", "ダウンロード KB/s", "下载 KB/s", "Загрузка КБ/с"]),
    ("rx_legend", ["수신 Mbps", "Received Mbps", "Reçu Mbit/s", "Empfangen Mbit/s", "受信 Mbps", "接收 Mbps", "Приём Мбит/с"]),
    ("tx_legend", ["송신 Mbps", "Sent Mbps", "Envoyé Mbit/s", "Gesendet Mbit/s", "送信 Mbps", "发送 Mbps", "Отправка Мбит/с"]),
    ("region_graph_title", ["지역별 다운로드 속도 그래프 (KB/s)", "Download speed by region (KB/s)", "Vitesse par région (Ko/s)", "Download-Geschwindigkeit nach Region (KB/s)", "地域別ダウンロード速度 (KB/s)", "各地区下载速度 (KB/s)", "Скорость загрузки по регионам (КБ/с)"]),
    ("region_table_title", ["지역별 다운로드 속도 (100KB · 5초마다)", "Download speed by region (100 KB · every 5 s)", "Vitesse par région (100 Ko · toutes les 5 s)", "Download-Geschwindigkeit nach Region (100 KB · alle 5 s)", "地域別ダウンロード速度 (100KB · 5秒ごと)", "各地区下载速度（100KB · 每5秒）", "Скорость по регионам (100 КБ · каждые 5 с)"]),
    ("region", ["지역", "Region", "Région", "Region", "地域", "地区", "Регион"]),
    ("status", ["상태", "Status", "État", "Status", "状態", "状态", "Статус"]),
    ("status_done", ["측정 완료 ({n}KB)", "Done ({n} KB)", "OK ({n} Ko)", "OK ({n} KB)", "完了 ({n}KB)", "完成 ({n}KB)", "Готово ({n} КБ)"]),
    ("status_unset", ["URL 미설정", "URL not set", "URL non définie", "URL nicht gesetzt", "URL 未設定", "未设置 URL", "URL не задан"]),
    ("base_name", ["기본 (Cloudflare)", "Default (Cloudflare)", "Par défaut (Cloudflare)", "Standard (Cloudflare)", "標準 (Cloudflare)", "默认 (Cloudflare)", "По умолчанию (Cloudflare)"]),
    ("region_us", ["미국 (LA)", "USA (LA)", "États-Unis (LA)", "USA (LA)", "アメリカ (LA)", "美国 (LA)", "США (LA)"]),
    ("region_jp", ["일본 (도쿄)", "Japan (Tokyo)", "Japon (Tokyo)", "Japan (Tokio)", "日本 (東京)", "日本 (东京)", "Япония (Токио)"]),
    ("region_sg", ["싱가포르", "Singapore", "Singapour", "Singapur", "シンガポール", "新加坡", "Сингапур"]),
    ("region_kr", ["한국 (daum.net)", "Korea (daum.net)", "Corée (daum.net)", "Korea (daum.net)", "韓国 (daum.net)", "韩国 (daum.net)", "Корея (daum.net)"]),
    ("region_cn", ["중국 (바이두 baidu.com)", "China (Baidu baidu.com)", "Chine (Baidu baidu.com)", "China (Baidu baidu.com)", "中国 (百度 baidu.com)", "中国 (百度 baidu.com)", "Китай (Baidu baidu.com)"]),
    // ----- 도움말 -----
    ("help_title", ["도움말 · MRPUZZLE-NETS", "Help · MRPUZZLE-NETS", "Aide · MRPUZZLE-NETS", "Hilfe · MRPUZZLE-NETS", "ヘルプ · MRPUZZLE-NETS", "帮助 · MRPUZZLE-NETS", "Справка · MRPUZZLE-NETS"]),
    ("help_heading", ["MRPUZZLE-NETS - 네트워크 모니터", "MRPUZZLE-NETS - Network Monitor", "MRPUZZLE-NETS - Moniteur réseau", "MRPUZZLE-NETS - Netzwerkmonitor", "MRPUZZLE-NETS - ネットワークモニター", "MRPUZZLE-NETS - 网络监控", "MRPUZZLE-NETS - Монитор сети"]),
    ("help_version", ["버전 {v}", "Version {v}", "Version {v}", "Version {v}", "バージョン {v}", "版本 {v}", "Версия {v}"]),
    ("help_author", ["제작자: MRPUZZLE (C)", "Author: MRPUZZLE (C)", "Auteur : MRPUZZLE (C)", "Autor: MRPUZZLE (C)", "制作者: MRPUZZLE (C)", "作者：MRPUZZLE (C)", "Автор: MRPUZZLE (C)"]),
    ("help_free", ["무료사용: 이 프로그램은 누구나 무료로 사용할 수 있습니다.", "Free to use: anyone may use this program at no cost.", "Gratuit : ce programme est libre d'utilisation.", "Kostenlos nutzbar: Dieses Programm darf von jedermann kostenlos verwendet werden.", "無料でご利用いただけます。誰でも無料で使用できます。", "免费使用：任何人均可免费使用本程序。", "Бесплатно: программой может пользоваться любой желающий."]),
    ("help_screens", ["화면 구성", "Screen layout", "Disposition de l'écran", "Bildschirmaufbau", "画面構成", "界面布局", "Структура экрана"]),
    ("help_s1", ["· 왼쪽 위: 내 네트워크 설정 (IP, 서브넷, 게이트웨이, DNS, MAC, 링크 속도 등)", "· Top left: my network settings (IP, subnet, gateway, DNS, MAC, link speed, etc.)", "· En haut à gauche : paramètres réseau (IP, sous-réseau, passerelle, DNS, MAC, vitesse…)", "· Oben links: Netzwerkeinstellungen (IP, Subnetz, Gateway, DNS, MAC, Geschwindigkeit usw.)", "· 左上: ネットワーク設定 (IP、サブネット、ゲートウェイ、DNS、MAC、リンク速度など)", "· 左上：网络设置（IP、子网、网关、DNS、MAC、链路速度等）", "· Слева вверху: настройки сети (IP, маска, шлюз, DNS, MAC, скорость и т. д.)"]),
    ("help_s2", ["· 왼쪽 가운데: 외부 연결 포트 (국가별 정렬), 5초 통신량, 포트별 누적 통신량", "· Middle left: external ports (sorted by country), 5-second traffic, per-port totals", "· Milieu gauche : ports externes (triés par pays), trafic 5 s, totaux par port", "· Mitte links: externe Ports (nach Land sortiert), 5-s-Verkehr, Summen je Port", "· 左中央: 外部接続ポート（国別順）、5秒通信量、ポート別累計", "· 左中：外部连接端口（按国家排序）、5秒流量、端口累计", "· Слева по центру: внешние порты (по странам), трафик за 5 с, суммы по портам"]),
    ("help_s3", ["· 왼쪽 아래: 내 공유 폴더와 SMB 접속 IP/이름", "· Bottom left: my shared folders and SMB connections (IP/name)", "· En bas à gauche : dossiers partagés et connexions SMB (IP/nom)", "· Unten links: meine Freigaben und SMB-Verbindungen (IP/Name)", "· 左下: 共有フォルダーと SMB 接続（IP/名前）", "· 左下：共享文件夹和 SMB 连接（IP/名称）", "· Слева внизу: мои общие папки и подключения SMB (IP/имя)"]),
    ("help_s4", ["· 가운데: 주변 네트워크 장치 (IP, 이름, 제조사 추정, MAC, 응답시간)", "· Center: nearby network devices (IP, name, estimated vendor, MAC, response time)", "· Centre : appareils réseau proches (IP, nom, fabricant estimé, MAC, temps de réponse)", "· Mitte: Geräte im Netzwerk (IP, Name, geschätzter Hersteller, MAC, Antwortzeit)", "· 中央: 周辺のネットワーク機器（IP、名前、推定メーカー、MAC、応答時間）", "· 中间：周边网络设备（IP、名称、推测厂商、MAC、响应时间）", "· По центру: устройства сети (IP, имя, предполагаемый производитель, MAC, отклик)"]),
    ("help_s5", ["· 오른쪽: 실시간 송수신 추산(Mbps), 외부 다운로드 속도(100KB, 5초마다), 지역별 속도 표와 그래프", "· Right: live send/receive estimate (Mbps), external download speed (100 KB, every 5 s), regional speed table and graph", "· Droite : estimation en direct (Mbit/s), vitesse de téléchargement externe (100 Ko, toutes les 5 s), tableau et graphe par région", "· Rechts: Live-Schätzung (Mbit/s), externe Download-Geschwindigkeit (100 KB, alle 5 s), Tabelle und Diagramm je Region", "· 右: リアルタイム送受信推定（Mbps）、外部ダウンロード速度（100KB、5秒ごと）、地域別速度表とグラフ", "· 右侧：实时收发估算（Mbps）、外部下载速度（100KB，每5秒）、各地区速度表和图表", "· Справа: оценка приёма и отправки (Мбит/с), скорость загрузки (100 КБ, каждые 5 с), таблица и график по регионам"]),
    ("help_s6", ["· 패널 경계를 드래그하면 크기가 바뀝니다. 바뀐 배치와 창 위치/크기는 nets_settings.ini 파일에 저장되어 다음 실행 때 그대로 나옵니다.", "· Drag panel borders to resize. The layout, window position and size are saved to nets_settings.ini and restored at next start.", "· Faites glisser les bords des panneaux pour les redimensionner. La disposition est enregistrée dans nets_settings.ini.", "· Panelränder ziehen zum Ändern der Größe. Layout, Fensterposition und -größe werden in nets_settings.ini gespeichert.", "· パネルの境界をドラッグするとサイズが変わります。配置・ウィンドウ位置・サイズは nets_settings.ini に保存され、次回起動時に復元されます。", "· 拖动面板边界可调整大小。布局、窗口位置和大小保存在 nets_settings.ini 中，下次启动时恢复。", "· Перетаскивайте границы панелей. Расположение, положение и размер окна сохраняются в nets_settings.ini и восстанавливаются при запуске."]),
    ("help_s7", ["· 지금 스캔: 주변 장치를 바로 다시 찾습니다. 장치 목록은 60초마다 자동 갱신됩니다.", "· Scan now: searches for nearby devices immediately. The list refreshes every 60 s.", "· Scanner : recherche immédiate des appareils. La liste est actualisée toutes les 60 s.", "· Jetzt scannen: sucht sofort nach Geräten. Die Liste wird alle 60 s aktualisiert.", "· 今すぐスキャン: 周辺デバイスをすぐに検索します。一覧は60秒ごとに自動更新されます。", "· 立即扫描：立刻重新查找周边设备。设备列表每60秒自动刷新。", "· Сканировать: сразу ищет устройства рядом. Список обновляется каждые 60 с."]),
    ("help_s8", ["· 마우스를 그래프 위에 올리면 각 지역의 해당 시점 속도가 표시됩니다.", "· Hover over the graph to see each region's speed at that moment.", "· Survolez le graphe pour voir la vitesse de chaque région à cet instant.", "· Über dem Diagramm zeigt die Anzeige die Geschwindigkeit jeder Region zu diesem Zeitpunkt.", "· グラフにカーソルを合わせると、その時点の各地域の速度が表示されます。", "· 将鼠标移到图表上，可查看该时刻各地区的速度。", "· Наведите курсор на график, чтобы увидеть скорость по регионам в этот момент."]),
    ("help_runtime", ["실행 조건", "Requirements", "Configuration requise", "Voraussetzungen", "動作条件", "运行条件", "Требования"]),
    ("help_r1", ["· Windows 10 / 11 (64비트)", "· Windows 10 / 11 (64-bit)", "· Windows 10 / 11 (64 bits)", "· Windows 10 / 11 (64-Bit)", "· Windows 10 / 11 (64ビット)", "· Windows 10 / 11（64 位）", "· Windows 10 / 11 (64-разрядная)"]),
    ("help_r2", ["· 관리자 권한 실행: 포트별 누적 통신량(TCP 연결 통계) 조회에 필요합니다. 실행 시 UAC 승인 창이 뜹니다.", "· Run as administrator: required for per-port totals (TCP connection statistics). A UAC prompt appears at start.", "· Exécution en administrateur : requise pour les totaux par port (statistiques TCP). Une invite UAC apparaît au lancement.", "· Als Administrator starten: nötig für Summen je Port (TCP-Verbindungsstatistik). Beim Start erscheint eine UAC-Abfrage.", "· 管理者として実行: ポート別累計（TCP接続統計）に必要です。起動時に UAC 確認が表示されます。", "· 以管理员身份运行：端口累计（TCP 连接统计）需要。启动时会弹出 UAC 确认。", "· Запуск от администратора: нужен для сумм по портам (статистика TCP). При запуске появится запрос UAC."]),
    ("help_r3", ["· 인터넷 연결: 외부 속도 측정(Cloudflare, Vultr, daum.net, baidu.com)과 국가 조회(ipinfo.io)에 필요합니다.", "· Internet connection: needed for external speed tests (Cloudflare, Vultr, daum.net, baidu.com) and country lookup (ipinfo.io).", "· Connexion Internet : requise pour les tests externes (Cloudflare, Vultr, daum.net, baidu.com) et la recherche de pays (ipinfo.io).", "· Internetverbindung: nötig für Geschwindigkeitstests (Cloudflare, Vultr, daum.net, baidu.com) und Länderabfrage (ipinfo.io).", "· インターネット接続: 外部速度測定（Cloudflare、Vultr、daum.net、baidu.com）と国の照会（ipinfo.io）に必要です。", "· 互联网连接：外部速度测试（Cloudflare、Vultr、daum.net、baidu.com）和国家查询（ipinfo.io）需要。", "· Подключение к Интернету: нужно для тестов скорости (Cloudflare, Vultr, daum.net, baidu.com) и определения страны (ipinfo.io)."]),
    ("help_r4", ["· 시스템 도구: ping, arp, netstat, PowerShell (Windows 기본 제공)", "· System tools: ping, arp, netstat, PowerShell (built into Windows)", "· Outils système : ping, arp, netstat, PowerShell (intégrés à Windows)", "· Systemwerkzeuge: ping, arp, netstat, PowerShell (in Windows enthalten)", "· システムツール: ping、arp、netstat、PowerShell（Windows 標準）", "· 系统工具：ping、arp、netstat、PowerShell（Windows 自带）", "· Системные утилиты: ping, arp, netstat, PowerShell (входят в Windows)"]),
    ("help_license", ["사용 라이선스 및 구성 요소", "Licenses and components", "Licences et composants", "Lizenzen und Komponenten", "ライセンスと構成要素", "许可证与组件", "Лицензии и компоненты"]),
    ("lic_rust", ["· Rust 언어 및 표준 라이브러리: MIT OR Apache-2.0", "· Rust language and standard library: MIT OR Apache-2.0", "· Langage Rust et bibliothèque standard : MIT OR Apache-2.0", "· Rust-Sprache und Standardbibliothek: MIT OR Apache-2.0", "· Rust 言語と標準ライブラリ: MIT OR Apache-2.0", "· Rust 语言及标准库：MIT OR Apache-2.0", "· Язык Rust и стандартная библиотека: MIT OR Apache-2.0"]),
    ("lic_egui", ["· eframe / egui (GUI): MIT OR Apache-2.0", "· eframe / egui (GUI): MIT OR Apache-2.0", "· eframe / egui (interface): MIT OR Apache-2.0", "· eframe / egui (GUI): MIT OR Apache-2.0", "· eframe / egui (GUI): MIT OR Apache-2.0", "· eframe / egui（界面）：MIT OR Apache-2.0", "· eframe / egui (GUI): MIT OR Apache-2.0"]),
    ("lic_plot", ["· egui_plot (그래프): MIT OR Apache-2.0", "· egui_plot (graphs): MIT OR Apache-2.0", "· egui_plot (graphiques) : MIT OR Apache-2.0", "· egui_plot (Diagramme): MIT OR Apache-2.0", "· egui_plot (グラフ): MIT OR Apache-2.0", "· egui_plot（图表）：MIT OR Apache-2.0", "· egui_plot (графики): MIT OR Apache-2.0"]),
    ("lic_ureq", ["· ureq (HTTP 요청): MIT OR Apache-2.0", "· ureq (HTTP requests): MIT OR Apache-2.0", "· ureq (requêtes HTTP) : MIT OR Apache-2.0", "· ureq (HTTP-Anfragen): MIT OR Apache-2.0", "· ureq (HTTP リクエスト): MIT OR Apache-2.0", "· ureq（HTTP 请求）：MIT OR Apache-2.0", "· ureq (HTTP-запросы): MIT OR Apache-2.0"]),
    ("lic_font", ["· 맑은 고딕 글꼴: Microsoft Windows 글꼴 (Windows 사용권 조건 적용)", "· Malgun Gothic font: Microsoft Windows font (Windows license terms apply)", "· Police Malgun Gothic : police Microsoft Windows (conditions de licence Windows)", "· Schriftart Malgun Gothic: Microsoft-Windows-Schrift (Windows-Lizenzbedingungen)", "· Malgun Gothic フォント: Microsoft Windows のフォント（Windows の使用許諾条件が適用）", "· 맑은 고딕字体：Microsoft Windows 字体（适用 Windows 许可条款）", "· Шрифт Malgun Gothic: шрифт Microsoft Windows (действуют условия лицензии Windows)"]),
    ("lic_services", ["· 외부 서비스: Cloudflare 속도 측정, Vultr 테스트 파일, daum.net, baidu.com, ipinfo.io (각 서비스 이용약관을 따릅니다)", "· External services: Cloudflare speed test, Vultr test files, daum.net, baidu.com, ipinfo.io (subject to each service's terms)", "· Services externes : test Cloudflare, fichiers Vultr, daum.net, baidu.com, ipinfo.io (conditions de chaque service)", "· Externe Dienste: Cloudflare-Test, Vultr-Testdateien, daum.net, baidu.com, ipinfo.io (es gelten die jeweiligen Nutzungsbedingungen)", "· 外部サービス: Cloudflare 速度測定、Vultr テストファイル、daum.net、baidu.com、ipinfo.io（各サービスの利用規約に従います）", "· 外部服务：Cloudflare 速度测试、Vultr 测试文件、daum.net、baidu.com、ipinfo.io（遵守各服务的使用条款）", "· Внешние сервисы: тест Cloudflare, файлы Vultr, daum.net, baidu.com, ipinfo.io (действуют условия каждого сервиса)"]),
    ("help_free_modify", ["무료 수정: 코드를 고치거나 개선하는 것도 자유입니다.", "Free to modify: you may also change and improve the code freely.", "Modification libre : vous pouvez aussi modifier et améliorer le code librement.", "Kostenlos änderbar: Der Code darf frei verändert und verbessert werden.", "自由に改変可能: コードの修正・改良も自由です。", "可自由修改：也可以自由修改和改进代码。", "Свободное изменение: код можно свободно исправлять и улучшать."]),
    ("help_attrib", ["필수 표기: 사용·수정·배포 시 'MRPUZZLE(C) NETS 에 기초하여 제작됨'을 반드시 명기해야 합니다.", "Attribution required: when using, modifying or redistributing, you must state that it is based on 'MRPUZZLE(C) NETS'.", "Attribution obligatoire : en cas d'utilisation, de modification ou de redistribution, indiquez qu'il est basé sur « MRPUZZLE(C) NETS ».", "Namensnennung erforderlich: Bei Nutzung, Änderung oder Weitergabe muss angegeben werden, dass es auf „MRPUZZLE(C) NETS“ basiert.", "表示義務: 使用・改変・再配布の際は「MRPUZZLE(C) NETS を基にしている」ことを明記してください。", "必须署名：使用、修改或再发布时，必须注明“基于 MRPUZZLE(C) NETS”。", "Обязательное указание авторства: при использовании, изменении или распространении нужно указать, что программа основана на «MRPUZZLE(C) NETS»."]),
    ("help_license_name", ["라이선스: 저작자 표시(Attribution) 조건의 무료 라이선스 (참고: Creative Commons CC BY 4.0, MIT)", "License: free license with attribution required (similar to Creative Commons CC BY 4.0, MIT)", "Licence : licence libre avec attribution obligatoire (similaire à Creative Commons CC BY 4.0, MIT)", "Lizenz: freie Lizenz mit Namensnennungspflicht (vergleichbar mit Creative Commons CC BY 4.0, MIT)", "ライセンス: 表示条件付きの無料ライセンス（参考: Creative Commons CC BY 4.0、MIT）", "许可证：带署名要求的免费许可（参考：Creative Commons CC BY 4.0、MIT）", "Лицензия: свободная лицензия с обязательным указанием авторства (аналоги: Creative Commons CC BY 4.0, MIT)"]),
    ("lic_note", ["위 라이선스는 각 구성 요소의 원 저작권자에게 있으며, 해당 조건을 따릅니다. 외부 서비스는 제공자 사정에 따라 중단되거나 값이 달라질 수 있습니다.", "These licenses belong to the original authors of each component and their terms apply. External services may stop or change values at the provider's discretion.", "Ces licences appartiennent aux auteurs d'origine de chaque composant. Les services externes peuvent être interrompus ou changer de valeurs.", "Diese Lizenzen gehören den ursprünglichen Urhebern der jeweiligen Komponenten. Externe Dienste können jederzeit unterbrochen werden oder abweichende Werte liefern.", "上記ライセンスは各構成要素の原著作権者に帰属し、その条件に従います。外部サービスは提供者の都合により停止または値が変わる場合があります。", "以上许可证归各组件的原作者所有，并遵循其条款。外部服务可能因提供方原因而中断或数值变化。", "Указанные лицензии принадлежат авторам соответствующих компонентов и действуют на их условиях. Внешние сервисы могут быть остановлены или изменить значения."]),
];
