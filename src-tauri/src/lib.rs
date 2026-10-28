pub mod core;
pub mod icon;
use std::sync::Mutex;
use tauri::State;
use tauri_plugin_shell::ShellExt;
use core::{AppItem, LauncherState};

struct AppState { launcher: Mutex<LauncherState> }

#[tauri::command]
fn get_state(state: State<'_, AppState>) -> LauncherState { state.launcher.lock().unwrap().clone() }

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState { launcher: Mutex::new(LauncherState::new(10)) })
        .invoke_handler(tauri::generate_handler![get_state, add_app_to_first_empty, launch_app])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
