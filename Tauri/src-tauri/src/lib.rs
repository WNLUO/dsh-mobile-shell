// 远程电脑桌面壳：校验一次性入口，只在本机同源窗口加载，外链交系统浏览器，网页下载走系统保存对话框，会话不落盘。
// Mac 上把原生窗口按钮落在网页左侧栏：窗口参数登记在配置里，运行时只换一次性地址。
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{Url, WebviewUrl};
use tauri::webview::{DownloadEvent, NewWindowResponse, WebviewWindowBuilder};
use tauri_plugin_dialog::DialogExt;

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

// 网页触发的下载先落到临时文件，下载完成后再弹系统保存对话框让用户选最终位置，取消则删掉临时文件。
// wry 只给同步的落盘决策（给不出对话框），所以先写到临时路径，完成后再让用户确认。
struct PendingDownload {
    temp: PathBuf,
    name: String,
    dir: Option<PathBuf>,
}

fn temp_download_path(name: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("ai-hub-download-{seq}-{name}"))
}

fn finish_download<R: tauri::Runtime>(app: &tauri::AppHandle<R>, pending: PendingDownload) {
    let PendingDownload { temp, name, dir } = pending;
    let save_from = temp.clone();
    let mut builder = app.dialog().file().set_file_name(name);
    if let Some(dir) = dir {
        builder = builder.set_directory(dir);
    }
    builder.save_file(move |chosen| {
        match chosen.and_then(|path| path.into_path().ok()) {
            Some(dest) => {
                // 拷贝成功才删临时文件，失败就留着临时文件，别把下载内容弄丢。
                if std::fs::copy(&save_from, &dest).is_ok() {
                    let _ = std::fs::remove_file(&save_from);
                }
            }
            None => {
                let _ = std::fs::remove_file(&save_from);
            }
        }
    });
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
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let downloads: Arc<Mutex<HashMap<String, Vec<PendingDownload>>>> =
                Arc::new(Mutex::new(HashMap::new()));
            let entry = url.clone();
            let mut config = app.config().app.windows[0].clone();
            config.url = WebviewUrl::External(url);
            config.title = title;
            // 与官方 Mac 桌面端一致：透明覆盖式标题栏、红黄绿按钮落在侧栏 (16, 18)、侧栏毛玻璃。
            #[cfg(target_os = "macos")]
            {
                config.title_bar_style = tauri::TitleBarStyle::Overlay;
                config.hidden_title = true;
                config.traffic_light_position = Some(tauri::utils::config::LogicalPosition { x: 16.0, y: 18.0 });
                config.transparent = true;
                config.window_effects = Some(tauri::utils::config::WindowEffectsConfig {
                    effects: vec![tauri::utils::WindowEffect::Sidebar],
                    state: Some(tauri::utils::WindowEffectState::Active),
                    ..Default::default()
                });
            }
            let window = WebviewWindowBuilder::from_config(app, &config)?
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
                .on_download(move |_webview, event| match event {
                    DownloadEvent::Requested { url, destination } => {
                        let name = destination
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "download".to_string());
                        let dir = destination.parent().map(|dir| dir.to_path_buf());
                        let temp = temp_download_path(&name);
                        downloads
                            .lock()
                            .unwrap()
                            .entry(url.to_string())
                            .or_default()
                            .push(PendingDownload { temp: temp.clone(), name, dir });
                        *destination = temp;
                        true
                    }
                    DownloadEvent::Finished { url, success, .. } => {
                        let pending = downloads
                            .lock()
                            .unwrap()
                            .get_mut(&url.to_string())
                            .and_then(|list| {
                                if list.is_empty() { None } else { Some(list.remove(0)) }
                            });
                        if let Some(pending) = pending {
                            if success {
                                finish_download(&app_handle, pending);
                            } else {
                                let _ = std::fs::remove_file(&pending.temp);
                            }
                        }
                        true
                    }
                    _ => true,
                })
                .build()?;
            // 原生全屏时红黄绿按钮隐藏，网页那块让位也要跟着收掉。
            #[cfg(target_os = "macos")]
            {
                let tracked = window.clone();
                window.on_window_event(move |event| {
                    if !matches!(event, tauri::WindowEvent::Resized(_)) { return; }
                    let fullscreen = tracked.is_fullscreen().unwrap_or(false);
                    let script = if fullscreen {
                        "document.documentElement.dataset.fullscreen='true';"
                    } else {
                        "delete document.documentElement.dataset.fullscreen;"
                    };
                    let _ = tracked.eval(script);
                });
            }
            #[cfg(not(target_os = "macos"))]
            let _ = &window;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
