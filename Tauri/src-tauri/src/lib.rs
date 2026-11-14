// 远程电脑桌面壳：校验一次性入口，只在本机同源窗口加载，外链交系统浏览器，会话不落盘。
use tauri::{Url, WebviewUrl};
use tauri::webview::{NewWindowResponse, WebviewWindowBuilder};

const REMOTE_URL_PREFIX: &str = "--remote-url=";
const DEVICE_NAME_PREFIX: &str = "--device-name=";
const LOOPBACK_HOST: &str = "127.0.0.1";

// 与插件前端 trustedRemoteUrl 同规则：只接受 http://127.0.0.1:<端口> 的一次性入口，拒绝其他协议、主机和账号密码。
fn trusted_remote_url(text: &str) -> Option<Url> {
    let url = Url::parse(text).ok()?;
    if url.scheme() != "http"
        || url.host_str() != Some(LOOPBACK_HOST)
        || url.port().is_none()
        || !url.username().is_empty()
        || !url.password().map_or(true, str::is_empty)
    {
        return None;
    }
    Some(url)
}

// 同源导航：协议、主机、端口都与一次性入口一致，且目标不带账号密码。
fn is_same_origin(entry: &Url, target: &Url) -> bool {
    entry.scheme() == target.scheme()
        && entry.host_str() == target.host_str()
        && entry.port() == target.port()
        && target.username().is_empty()
        && target.password().map_or(true, str::is_empty)
}

// 外链只认不带账号密码的 http(s) 外部地址；本机回环地址一律不外送，避免把一次性入口带出本窗口。
fn is_external_link(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || !url.password().map_or(true, str::is_empty)
    {
        return false;
    }
    !matches!(
        url.host_str(),
        Some("127.0.0.1") | Some("localhost") | Some("[::1]") | Some("::1")
    )
}

// 外链交给系统默认浏览器；系统浏览器不带本窗口的 HttpOnly 会话，也不共享一次性凭据。
fn open_in_system_browser(url: &Url) {
    let _ = tauri_plugin_opener::open_url(url.as_str(), None::<&str>);
}

fn window_title(device_name: &str) -> String {
    let name = device_name.trim();
    if name.is_empty() {
        "AI-HUB".to_string()
    } else {
        format!("AI-HUB · {name}")
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // 入口缺失或不合法就报错退出，绝不加载，也不把地址写进日志。
    let Some(url) = args
        .iter()
        .find_map(|arg| arg.strip_prefix(REMOTE_URL_PREFIX))
        .and_then(trusted_remote_url)
    else {
        eprintln!("启动失败：--remote-url 必须是 http://127.0.0.1:<端口> 的一次性地址，且不含账号密码。");
        std::process::exit(1);
    };
    let title = args
        .iter()
        .find_map(|arg| arg.strip_prefix(DEVICE_NAME_PREFIX))
        .map(window_title)
        .unwrap_or_else(|| "AI-HUB".to_string());

    tauri::Builder::default()
        .setup(move |app| {
            let entry = url.clone();
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title(title)
                // 与官方桌面端主窗口一致：默认 1280×820，最小 520×600。
                .inner_size(1280.0, 820.0)
                .min_inner_size(520.0, 600.0)
                .incognito(true)
                // 原生拖放处理器会吞掉 drop 事件，关掉才能让网页收到从访达拖入的文件。
                .disable_drag_drop_handler()
                .on_navigation(move |target| {
                    if is_same_origin(&entry, target) {
                        return true;
                    }
                    if is_external_link(target) {
                        open_in_system_browser(target);
                    }
                    false
                })
                .on_new_window(|target, _features| {
                    if is_external_link(&target) {
                        open_in_system_browser(&target);
                    }
                    NewWindowResponse::Deny
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
