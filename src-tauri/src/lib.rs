pub mod core;
pub mod favicon;
pub mod icon;
pub mod link;
pub mod menu;
pub mod net;
pub mod service;
pub mod settings;
pub mod storage;
pub mod window;
use std::sync::Mutex;
use tauri::{Emitter, Manager, State};
use tauri_plugin_autostart::{MacosLauncher, ManagerExt};
use tauri_plugin_shell::ShellExt;
use core::{AppItem, LauncherState};
use service::LauncherService;
use settings::SettingsService;
use storage::{StateStore, StdFs};

struct AppState { launcher: Mutex<LauncherService<StdFs>>, settings: Mutex<SettingsService<StdFs>> }

#[tauri::command]
fn get_state(state: State<'_, AppState>) -> LauncherState { state.launcher.lock().unwrap().state().clone() }

/// `.app` のパスからアイテムを作る。表示名は拡張子を除いたファイル名。
fn app_item(path: String) -> AppItem {
    let name = std::path::Path::new(&path).file_stem().unwrap_or_default().to_string_lossy().to_string();
    AppItem::new(path, name)
}

#[tauri::command]
fn add_app_to_first_empty(page_idx: usize, path: String, state: State<'_, AppState>) -> Result<(), String> {
    state.launcher.lock().unwrap().add_app_to_first_empty(page_idx, app_item(path))
}

/// ドロップ位置のスロットに差し込む。埋まっていれば後ろへずらす。
#[tauri::command]
fn insert_app(page_idx: usize, slot_idx: usize, path: String, state: State<'_, AppState>) -> Result<(), String> {
    state.launcher.lock().unwrap().insert_app(page_idx, slot_idx, app_item(path))
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

/// ドロップされた `.webloc` / `.url` を読み、中の URL のアイテムを作る。
async fn url_file_item(path: String) -> Result<AppItem, String> {
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
    build_url_item(link::shortcut_url(&file, &data)?).await
}

#[tauri::command]
async fn add_url_file_to_first_empty(page_idx: usize, path: String, state: State<'_, AppState>) -> Result<(), String> {
    let item = url_file_item(path).await?;
    state.launcher.lock().unwrap().add_app_to_first_empty(page_idx, item)
}

#[tauri::command]
async fn insert_url_file(page_idx: usize, slot_idx: usize, path: String, state: State<'_, AppState>) -> Result<(), String> {
    let item = url_file_item(path).await?;
    state.launcher.lock().unwrap().insert_app(page_idx, slot_idx, item)
}

#[tauri::command]
fn rename_app(page_idx: usize, slot_idx: usize, name: String, state: State<'_, AppState>) -> Result<(), String> {
    state.launcher.lock().unwrap().rename_app(page_idx, slot_idx, &name)
}

#[tauri::command]
fn remove_app(page_idx: usize, slot_idx: usize, state: State<'_, AppState>) -> Result<(), String> {
    state.launcher.lock().unwrap().remove_app(page_idx, slot_idx)
}

#[tauri::command]
fn move_app(page_idx: usize, from_slot: usize, to_slot: usize, state: State<'_, AppState>) -> Result<(), String> {
    state.launcher.lock().unwrap().move_app(page_idx, from_slot, to_slot)
}

/// 設定ウィンドウに見せる値。「起動時に実行する」は OS への登録状態そのものなので、保存せず毎回問い合わせる。
#[derive(serde::Serialize)]
struct SettingsView { always_on_top: bool, launch_at_login: bool }

#[tauri::command]
fn get_settings(app: tauri::AppHandle, state: State<'_, AppState>) -> SettingsView {
    SettingsView {
        always_on_top: state.settings.lock().unwrap().settings().always_on_top,
        launch_at_login: app.autolaunch().is_enabled().unwrap_or(false),
    }
}

/// ログイン時に起動するよう OS (~/Library/LaunchAgents) に登録・解除する。
#[tauri::command]
fn set_launch_at_login(enabled: bool, app: tauri::AppHandle) -> Result<(), String> {
    let autolaunch = app.autolaunch();
    let result = if enabled { autolaunch.enable() } else { autolaunch.disable() };
    result.map_err(|e| e.to_string())
}

/// 保存できてからメインウィンドウに反映する (次回起動時の状態と食い違わないように)。
/// 設定ウィンドウから呼ばれるので、呼び出し元ではなくメインウィンドウを対象にする。
#[tauri::command]
fn set_always_on_top(enabled: bool, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    state.settings.lock().unwrap().set_always_on_top(enabled)?;
    let main = app.get_webview_window("main").ok_or("main window not found")?;
    main.set_always_on_top(enabled).map_err(|e| e.to_string())
}

/// 設定ウィンドウに見せるグリッドの大きさとページ数。
#[derive(serde::Serialize)]
struct LayoutView { cols: usize, rows: usize, pages: usize }

#[tauri::command]
fn get_layout(state: State<'_, AppState>) -> LayoutView {
    let launcher = state.launcher.lock().unwrap();
    let s = launcher.state();
    LayoutView { cols: s.cols, rows: s.rows, pages: s.max_pages }
}

/// グリッドの大きさとページ数を変え、マスの大きさを保つようにメインウィンドウの大きさを合わせる。
/// 設定ウィンドウから呼ばれるので、メインウィンドウには再読み込みを知らせる。
#[tauri::command]
fn set_layout(cols: usize, rows: usize, pages: usize, app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let main = app.get_webview_window("main").ok_or("main window not found")?;
    let mut launcher = state.launcher.lock().unwrap();
    let from = (launcher.state().cols, launcher.state().rows);
    launcher.set_layout(cols, rows, pages)?;
    drop(launcher);
    let resize = || -> tauri::Result<()> {
        let width = main.inner_size()?.to_logical::<f64>(main.scale_factor()?).width;
        let size = window::resize_for_layout(width, from, (cols, rows), screen_size(&main)?);
        fit_window_to_grid(&main, cols, rows, size)
    };
    resize().map_err(|e| e.to_string())?;
    main.emit("launcher-changed", ()).map_err(|e| e.to_string())
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

#[tauri::command]
fn get_app_icon_stamp(path: String) -> Result<String, String> {
    icon::icon_stamp(std::path::Path::new(&path))
}

/// 覚えておいた設定 (ウィンドウ幅など) を保存する。終了処理の途中なので、失敗しても止めない。
fn flush_settings<R: tauri::Runtime>(app: &impl Manager<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        if let Err(e) = state.settings.lock().unwrap().flush() { eprintln!("failed to save settings: {e}"); }
    }
}

/// 今いる画面のうち、メニューバーと Dock を除いて使える大きさ (論理ピクセル)。
fn screen_size(window: &tauri::WebviewWindow) -> tauri::Result<Option<(f64, f64)>> {
    Ok(window.current_monitor()?.map(|m| {
        let size = m.work_area().size.to_logical::<f64>(m.scale_factor());
        (size.width, size.height)
    }))
}

/// グリッドの大きさに合わせて、最小サイズ・縦横比・ウィンドウの大きさを設定する。
fn fit_window_to_grid(window: &tauri::WebviewWindow, cols: usize, rows: usize, (width, height): (f64, f64)) -> tauri::Result<()> {
    let (min_w, min_h) = window::min_size(cols, rows);
    window.set_min_size(Some(tauri::LogicalSize::new(min_w, min_h)))?;
    #[cfg(target_os = "macos")]
    lock_aspect_ratio(window, cols, rows)?;
    window.set_size(tauri::LogicalSize::new(width, height))
}

/// リサイズ中も縦横比を保つ (tao には API が無いので NSWindow に直接設定する)。
#[cfg(target_os = "macos")]
fn lock_aspect_ratio(window: &tauri::WebviewWindow, cols: usize, rows: usize) -> tauri::Result<()> {
    let window = window.clone();
    let (w, h) = window::base_size(cols, rows);
    window.clone().run_on_main_thread(move || {
        use objc2_app_kit::NSWindow;
        use objc2_foundation::NSSize;
        let Ok(ptr) = window.ns_window() else { return };
        // SAFETY: ns_window() は生きている NSWindow を指し、ここはメインスレッドで実行される。
        let ns_window = unsafe { &*(ptr as *mut NSWindow) };
        ns_window.setContentAspectRatio(NSSize::new(w, h));
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None))
        .menu(menu::build)
        .on_menu_event(|app, event| menu::handle_event(app, event.id().as_ref()))
        // 覚えるのはメインウィンドウの大きさだけ (設定ウィンドウは対象外)
        .on_window_event(|window, event| match event {
            _ if window.label() != "main" => {}
            // 最小化などで 0 になる場合は覚えない
            tauri::WindowEvent::Resized(size) if size.width > 0 => {
                let scale = window.scale_factor().unwrap_or(1.0);
                if let Some(state) = window.try_state::<AppState>() {
                    state.settings.lock().unwrap().remember_window_width(size.to_logical::<f64>(scale).width);
                }
            }
            tauri::WindowEvent::CloseRequested { .. } => flush_settings(window),
            _ => {}
        })
        .setup(|app| {
            let window = app.get_webview_window("main").ok_or("main window not found")?;

            // 状態と設定は OS 標準のアプリデータ領域 (~/Library/Application Support/<identifier>) に保存する
            let data_dir = app.path().app_data_dir()?;
            let launcher = LauncherService::open(StateStore::new(StdFs, data_dir.join("state.json")));
            let settings = SettingsService::open(StdFs, data_dir.join("settings.json"));
            window.set_always_on_top(settings.settings().always_on_top)?;
            // 前回のウィンドウ幅で開く。画面より大きくならないよう、今いる画面に収める。
            let (cols, rows) = (launcher.state().cols, launcher.state().rows);
            let size = window::restore_size(settings.settings().window_width, cols, rows, screen_size(&window)?);
            fit_window_to_grid(&window, cols, rows, size)?;
            app.manage(AppState { launcher: Mutex::new(launcher), settings: Mutex::new(settings) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            add_app_to_first_empty,
            add_url_to_first_empty,
            add_url_to_slot,
            add_url_file_to_first_empty,
            launch_app,
            get_app_icon,
            get_app_icon_stamp,
            get_settings,
            get_layout,
            set_layout,
            set_always_on_top,
            set_launch_at_login,
            rename_app,
            remove_app,
            move_app,
            insert_app,
            insert_url_file
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        // ⌘Q での終了はウィンドウの CloseRequested を経ないので、ここでも保存する
        .run(|app, event| if let tauri::RunEvent::Exit = event { flush_settings(app) });
}
