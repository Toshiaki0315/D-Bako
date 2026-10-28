pub mod core;
pub mod favicon;
pub mod icon;
pub mod link;
pub mod net;
pub mod service;
pub mod storage;
pub mod window;
use std::sync::Mutex;
use tauri::{Manager, State};
use tauri_plugin_shell::ShellExt;
use core::{AppItem, LauncherState};
use service::LauncherService;
use storage::{StateStore, StdFs};

struct AppState { launcher: Mutex<LauncherService<StdFs>> }

#[tauri::command]
fn get_state(state: State<'_, AppState>) -> LauncherState { state.launcher.lock().unwrap().state().clone() }

#[tauri::command]
fn add_app_to_first_empty(page_idx: usize, path: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut launcher = state.launcher.lock().unwrap();
    let name = std::path::Path::new(&path).file_stem().unwrap_or_default().to_string_lossy().to_string();
    launcher.add_app_to_first_empty(page_idx, AppItem::new(path, name))
}

#[tauri::command]
async fn launch_app(app: tauri::AppHandle, path: String) -> Result<(), String> {
    if !link::is_launchable(&path) { return Err("Unsupported launch target".to_string()); }
    let _ = app.shell().command("open").arg(&path).spawn().map_err(|e| e.to_string())?; Ok(())
}

/// 入力を URL として検証し、favicon を取得してアイテムにする。通信は別スレッドで行い、UI を塞がない。
async fn build_url_item(input: String) -> Result<AppItem, String> {
    let url = link::normalize_url(&input)?;
    tauri::async_runtime::spawn_blocking(move || {
        let icon = favicon::fetch_favicon(&net::UreqClient::new(), &url);
        link::url_item(&url, icon)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_url_to_first_empty(page_idx: usize, input: String, state: State<'_, AppState>) -> Result<(), String> {
    let item = build_url_item(input).await?;
    state.launcher.lock().unwrap().add_app_to_first_empty(page_idx, item)
}

#[tauri::command]
async fn add_url_to_slot(page_idx: usize, slot_idx: usize, input: String, state: State<'_, AppState>) -> Result<(), String> {
    let item = build_url_item(input).await?;
    state.launcher.lock().unwrap().add_app_to_slot(page_idx, slot_idx, item)
}

/// ドロップされた `.webloc` / `.url` を読み、中の URL を登録する。
#[tauri::command]
async fn add_url_file_to_first_empty(page_idx: usize, path: String, state: State<'_, AppState>) -> Result<(), String> {
    const MAX_SHORTCUT_BYTES: u64 = 1024 * 1024;
    let file = std::path::PathBuf::from(path);
    let read_path = file.clone();
    let data = tauri::async_runtime::spawn_blocking(move || {
        let size = std::fs::metadata(&read_path).map_err(|e| e.to_string())?.len();
        if size > MAX_SHORTCUT_BYTES { return Err("Shortcut file is too large".to_string()); }
        std::fs::read(&read_path).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    let item = build_url_item(link::shortcut_url(&file, &data)?).await?;
    state.launcher.lock().unwrap().add_app_to_first_empty(page_idx, item)
}

/// アイコン抽出はファイル I/O を伴うため、UI/非同期ランタイムを塞がないよう別スレッドで実行する。
#[tauri::command]
async fn get_app_icon(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        icon::extract_icon_png(std::path::Path::new(&path)).map(|png| icon::to_data_url(&png))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// リサイズ中も縦横比を保つ (tao には API が無いので NSWindow に直接設定する)。
#[cfg(target_os = "macos")]
fn lock_aspect_ratio(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    use objc2_app_kit::NSWindow;
    use objc2_foundation::NSSize;
    let ptr = window.ns_window()? as *mut NSWindow;
    // SAFETY: ns_window() は生きている NSWindow を指す。setup はメインスレッドで呼ばれる。
    let ns_window = unsafe { &*ptr };
    ns_window.setContentAspectRatio(NSSize::new(window::BASE_WIDTH, window::BASE_HEIGHT));
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let window = app.get_webview_window("main").ok_or("main window not found")?;
            let (min_w, min_h) = window::min_size();
            window.set_min_size(Some(tauri::LogicalSize::new(min_w, min_h)))?;
            #[cfg(target_os = "macos")]
            lock_aspect_ratio(&window)?;

            // 状態は OS 標準のアプリデータ領域 (~/Library/Application Support/<identifier>) に保存する
            let path = app.path().app_data_dir()?.join("state.json");
            let launcher = LauncherService::open(StateStore::new(StdFs, path));
            app.manage(AppState { launcher: Mutex::new(launcher) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            add_app_to_first_empty,
            add_url_to_first_empty,
            add_url_to_slot,
            add_url_file_to_first_empty,
            launch_app,
            get_app_icon
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
