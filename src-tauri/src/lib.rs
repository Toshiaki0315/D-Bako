pub mod core;
pub mod icon;
pub mod service;
pub mod storage;
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
    launcher.add_app_to_first_empty(page_idx, AppItem { path, name })
}

#[tauri::command]
async fn launch_app(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let _ = app.shell().command("open").arg(&path).spawn().map_err(|e| e.to_string())?; Ok(())
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // 状態は OS 標準のアプリデータ領域 (~/Library/Application Support/<identifier>) に保存する
            let path = app.path().app_data_dir()?.join("state.json");
            let launcher = LauncherService::open(StateStore::new(StdFs, path));
            app.manage(AppState { launcher: Mutex::new(launcher) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_state, add_app_to_first_empty, launch_app, get_app_icon])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
