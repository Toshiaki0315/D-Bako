//! メニューバー (日本語表記)。Tauri 標準のメニューは英語固定なので、macOS の日本語表記に合わせて組み立てる。

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Emitter, Runtime};

/// 「設定…」のメニュー ID。
pub const SETTINGS_ID: &str = "settings";
/// 「設定…」が選ばれたときにフロントエンドへ送るイベント。
pub const OPEN_SETTINGS_EVENT: &str = "open-settings";

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

/// メニューの選択を処理する。設定画面はフロントエンドのダイアログなので、開くよう通知する。
pub fn handle_event<R: Runtime>(app: &AppHandle<R>, id: &str) {
    if id == SETTINGS_ID {
        let _ = app.emit_to("main", OPEN_SETTINGS_EVENT, ());
    }
}
