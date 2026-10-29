//! メニューバー (日本語表記)。Tauri 標準のメニューは英語固定なので、macOS の日本語表記に合わせて組み立てる。

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, PhysicalPosition, Runtime, Theme, WebviewUrl, WebviewWindowBuilder};

/// 「設定…」のメニュー ID。
pub const SETTINGS_ID: &str = "settings";
/// 設定ウィンドウのラベル。フロントエンドは `?view=settings` を見て設定画面を表示する。
pub const SETTINGS_WINDOW: &str = "settings";

pub fn build<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    let app_menu = Submenu::with_items(
        app,
        "道具箱",
        true,
        &[
            &PredefinedMenuItem::about(app, Some("道具箱について"), None)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, SETTINGS_ID, "設定…", true, Some("CmdOrCtrl+,"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::services(app, Some("サービス"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::hide(app, Some("道具箱を非表示"))?,
            &PredefinedMenuItem::hide_others(app, Some("ほかを非表示"))?,
            &PredefinedMenuItem::show_all(app, Some("すべてを表示"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::quit(app, Some("道具箱を終了"))?,
        ],
    )?;
    // 標準の編集メニューが無いと、入力欄で ⌘C / ⌘V などが効かない
    let edit_menu = Submenu::with_items(
        app,
        "編集",
        true,
        &[
            &PredefinedMenuItem::undo(app, Some("取り消す"))?,
            &PredefinedMenuItem::redo(app, Some("やり直す"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, Some("カット"))?,
            &PredefinedMenuItem::copy(app, Some("コピー"))?,
            &PredefinedMenuItem::paste(app, Some("ペースト"))?,
            &PredefinedMenuItem::select_all(app, Some("すべてを選択"))?,
        ],
    )?;
    let window_menu = Submenu::with_items(
        app,
        "ウインドウ",
        true,
        &[
            &PredefinedMenuItem::minimize(app, Some("しまう"))?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::close_window(app, Some("閉じる"))?,
        ],
    )?;
    Menu::with_items(app, &[&app_menu, &edit_menu, &window_menu])
}

/// メニューの選択を処理する。
pub fn handle_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    if id == SETTINGS_ID {
        if let Err(e) = open_settings(app) { eprintln!("failed to open settings: {e}"); }
    }
}

/// 設定ウィンドウを開く。既に開いていれば手前に出す。
/// メインウィンドウの子にして、メインが「常に前に表示」でも設定ウィンドウが後ろに隠れないようにする。
/// 子ウィンドウは親と別のディスプレイにあると表示されないので、メインウィンドウのあるディスプレイの中央に開く。
fn open_settings<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS_WINDOW) { return window.set_focus(); }
    let main = app.get_webview_window("main");
    let mut builder = WebviewWindowBuilder::new(app, SETTINGS_WINDOW, WebviewUrl::App("index.html?view=settings".into()))
        .title("設定")
        .inner_size(360.0, 160.0)
        .resizable(false)
        .minimizable(false)
        .maximizable(false)
        .theme(Some(Theme::Dark))
        .visible(false)
        .center();
    if let Some(main) = &main { builder = builder.parent(main)?; }
    let window = builder.build()?;
    if let Some(monitor) = main.as_ref().map(|m| m.current_monitor()).transpose()?.flatten() {
        let area = monitor.work_area();
        let size = window.outer_size()?;
        let (x, y) = crate::window::center_in((area.position.x, area.position.y), (area.size.width, area.size.height), (size.width, size.height));
        window.set_position(PhysicalPosition::new(x, y))?;
    }
    window.show()?;
    window.set_focus()
}
