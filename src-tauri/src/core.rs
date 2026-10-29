use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppItem {
    /// `.app` のパス。`kind` が `Url` のときは URL。
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub kind: ItemKind,
    /// URL ショートカットの favicon (data URL)。取得できなければ None。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind { #[default] App, Url }
impl AppItem {
    pub fn new(path: impl Into<String>, name: impl Into<String>) -> Self {
        Self { path: path.into(), name: name.into(), kind: ItemKind::App, icon: None }
    }
    pub fn url(url: impl Into<String>, name: impl Into<String>, icon: Option<String>) -> Self {
        Self { path: url.into(), name: name.into(), kind: ItemKind::Url, icon }
    }
}
/// 表示名の最大文字数。吹き出しに収まらない長さは省略表示になる。
pub const MAX_NAME_CHARS: usize = 100;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherState { pub pages: Vec<Vec<Option<AppItem>>>, pub max_pages: usize }
impl LauncherState {
    pub fn new(max_pages: usize) -> Self {
        let max_pages = max_pages.clamp(1, 10);
        let mut pages = Vec::with_capacity(max_pages);
        for _ in 0..max_pages { pages.push(vec![None; 8]); }
        Self { pages, max_pages }
    }
    /// 永続化用に JSON 文字列へ変換する。
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
    /// JSON から復元する。ファイルは外部入力なので不変条件（ページ数・スロット数）を検証する。
    pub fn from_json(json: &str) -> Result<Self, String> {
        let state: Self = serde_json::from_str(json).map_err(|e| e.to_string())?;
        if !(1..=10).contains(&state.max_pages) { return Err("Invalid max_pages".to_string()); }
        if state.pages.len() != state.max_pages { return Err("Page count does not match max_pages".to_string()); }
        if state.pages.iter().any(|p| p.len() != 8) { return Err("Each page must have 8 slots".to_string()); }
        Ok(state)
    }
    fn check_pos(&self, page_idx: usize, slot_idx: usize) -> Result<(), String> {
        if page_idx >= self.max_pages { return Err("Invalid page index".to_string()); }
        if slot_idx >= 8 { return Err("Invalid slot index".to_string()); }
        Ok(())
    }
    pub fn add_app(&mut self, page_idx: usize, slot_idx: usize, app: AppItem) -> Result<(), String> {
        self.check_pos(page_idx, slot_idx)?;
        self.pages[page_idx][slot_idx] = Some(app);
        Ok(())
    }
    /// 指定スロットが空の場合だけ登録する。埋まっていればエラー (上書きしない)。
    pub fn add_app_to_empty_slot(&mut self, page_idx: usize, slot_idx: usize, app: AppItem) -> Result<(), String> {
        self.check_pos(page_idx, slot_idx)?;
        if self.pages[page_idx][slot_idx].is_some() { return Err("Slot is occupied".to_string()); }
        self.pages[page_idx][slot_idx] = Some(app);
        Ok(())
    }
    /// 指定スロットのアプリを削除して返す。空スロットはエラー。
    pub fn remove_app(&mut self, page_idx: usize, slot_idx: usize) -> Result<AppItem, String> {
        self.check_pos(page_idx, slot_idx)?;
        self.pages[page_idx][slot_idx].take().ok_or_else(|| "Slot is empty".to_string())
    }
    /// `from` のアプリを `to` へ移動する。移動先が埋まっている場合は入れ替える。
    pub fn move_app(&mut self, from: (usize, usize), to: (usize, usize)) -> Result<(), String> {
        self.check_pos(from.0, from.1)?;
        self.check_pos(to.0, to.1)?;
        if self.pages[from.0][from.1].is_none() { return Err("Slot is empty".to_string()); }
        if from == to { return Ok(()); }
        let src = self.pages[from.0][from.1].take();
        let dst = std::mem::replace(&mut self.pages[to.0][to.1], src);
        self.pages[from.0][from.1] = dst;
        Ok(())
    }
    /// 指定スロットのアイテムの表示名を変える。前後の空白は除き、空や長すぎる名前は拒否する。
    pub fn rename_app(&mut self, page_idx: usize, slot_idx: usize, name: &str) -> Result<(), String> {
        self.check_pos(page_idx, slot_idx)?;
        let name = name.trim();
        if name.is_empty() { return Err("Name is empty".to_string()); }
        if name.chars().count() > MAX_NAME_CHARS { return Err("Name is too long".to_string()); }
        let item = self.pages[page_idx][slot_idx].as_mut().ok_or_else(|| "Slot is empty".to_string())?;
        item.name = name.to_string();
        Ok(())
    }
    /// 指定スロットに差し込む。埋まっていれば、そこから同じページの次の空きまでを 1 つずつ後ろへずらす。
    pub fn insert_app(&mut self, page_idx: usize, slot_idx: usize, app: AppItem) -> Result<(), String> {
        self.check_pos(page_idx, slot_idx)?;
        let page = &mut self.pages[page_idx];
        let empty = page[slot_idx..].iter().position(|s| s.is_none())
            .ok_or_else(|| "No empty slot after this position".to_string())?;
        // 空きを先頭へ回してから埋める
        page[slot_idx..=slot_idx + empty].rotate_right(1);
        page[slot_idx] = Some(app);
        Ok(())
    }
    pub fn add_app_to_first_empty(&mut self, page_idx: usize, app: AppItem) -> Result<(), String> {
        if page_idx >= self.max_pages { return Err("Invalid page index".to_string()); }
        if let Some(slot) = self.pages[page_idx].iter_mut().find(|s| s.is_none()) {
            *slot = Some(app); Ok(())
        } else { Err("No empty slot on this page".to_string()) }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_initial_state() { let state = LauncherState::new(3); assert_eq!(state.max_pages, 3); assert!(state.pages[0].iter().all(|slot| slot.is_none())); }
    #[test] fn test_max_pages_limit() { let state = LauncherState::new(15); assert_eq!(state.max_pages, 10); }
    #[test] fn test_add_app() { let mut state = LauncherState::new(1); let app = AppItem::new("/A.app", "A"); assert!(state.add_app(0, 0, app.clone()).is_ok()); assert_eq!(state.pages[0][0], Some(app)); }
    fn item(n: &str) -> AppItem { AppItem::new(format!("/{n}.app"), n) }
    #[test] fn test_new_clamps_zero_pages_to_one() { assert_eq!(LauncherState::new(0).max_pages, 1); }
    #[test] fn test_add_app_rejects_invalid_page() { let mut s = LauncherState::new(1); assert_eq!(s.add_app(1, 0, item("A")), Err("Invalid page index".to_string())); }
    #[test] fn test_add_app_rejects_invalid_slot() { let mut s = LauncherState::new(1); assert_eq!(s.add_app(0, 8, item("A")), Err("Invalid slot index".to_string())); }
    #[test] fn test_add_to_first_empty_fills_in_order() {
        let mut s = LauncherState::new(1);
        s.add_app_to_first_empty(0, item("A")).unwrap();
        s.add_app_to_first_empty(0, item("B")).unwrap();
        assert_eq!(s.pages[0][0], Some(item("A")));
        assert_eq!(s.pages[0][1], Some(item("B")));
    }
    #[test] fn test_add_to_first_empty_rejects_invalid_page() { let mut s = LauncherState::new(1); assert_eq!(s.add_app_to_first_empty(5, item("A")), Err("Invalid page index".to_string())); }
    #[test] fn test_json_roundtrip_preserves_state() {
        let mut s = LauncherState::new(3);
        s.add_app(2, 7, item("A")).unwrap();
        let restored = LauncherState::from_json(&s.to_json().unwrap()).unwrap();
        assert_eq!(restored.max_pages, 3);
        assert_eq!(restored.pages, s.pages);
    }
    #[test] fn test_from_json_rejects_malformed_json() { assert!(LauncherState::from_json("not json").is_err()); }
    #[test] fn test_from_json_rejects_out_of_range_max_pages() {
        assert_eq!(LauncherState::from_json(r#"{"pages":[],"max_pages":0}"#).unwrap_err(), "Invalid max_pages");
        let eleven = format!(r#"{{"pages":[{}],"max_pages":11}}"#, vec![r#"[null,null,null,null,null,null,null,null]"#; 11].join(","));
        assert_eq!(LauncherState::from_json(&eleven).unwrap_err(), "Invalid max_pages");
    }
    #[test] fn test_from_json_rejects_page_count_mismatch() {
        let json = r#"{"pages":[[null,null,null,null,null,null,null,null]],"max_pages":2}"#;
        assert_eq!(LauncherState::from_json(json).unwrap_err(), "Page count does not match max_pages");
    }
    #[test] fn test_from_json_rejects_wrong_slot_count() {
        let json = r#"{"pages":[[null,null]],"max_pages":1}"#;
        assert_eq!(LauncherState::from_json(json).unwrap_err(), "Each page must have 8 slots");
    }
    #[test] fn test_add_to_empty_slot_places_the_item() {
        let mut s = LauncherState::new(2);
        assert!(s.add_app_to_empty_slot(1, 4, item("A")).is_ok());
        assert_eq!(s.pages[1][4], Some(item("A")));
    }
    #[test] fn test_add_to_empty_slot_rejects_an_occupied_slot_without_overwriting() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 0, item("A")).unwrap();
        assert_eq!(s.add_app_to_empty_slot(0, 0, item("B")), Err("Slot is occupied".to_string()));
        assert_eq!(s.pages[0][0], Some(item("A")));
    }
    #[test] fn test_add_to_empty_slot_rejects_invalid_positions() {
        let mut s = LauncherState::new(1);
        assert_eq!(s.add_app_to_empty_slot(1, 0, item("A")), Err("Invalid page index".to_string()));
        assert_eq!(s.add_app_to_empty_slot(0, 8, item("A")), Err("Invalid slot index".to_string()));
    }
    #[test] fn test_app_item_new_defaults_to_app_kind() {
        let i = AppItem::new("/A.app", "A");
        assert_eq!(i.kind, ItemKind::App);
        assert_eq!(i.icon, None);
    }
    #[test] fn test_url_item_keeps_kind_and_icon_through_json() {
        let mut s = LauncherState::new(1);
        let it = AppItem::url("https://example.com/", "example.com", Some("data:image/png;base64,AA".into()));
        assert_eq!(it.kind, ItemKind::Url);
        s.add_app(0, 0, it.clone()).unwrap();
        let restored = LauncherState::from_json(&s.to_json().unwrap()).unwrap();
        assert_eq!(restored.pages[0][0], Some(it));
    }
    #[test] fn test_legacy_json_without_kind_loads_as_app() {
        let json = r#"{"pages":[[{"path":"/A.app","name":"A"},null,null,null,null,null,null,null]],"max_pages":1}"#;
        let s = LauncherState::from_json(json).unwrap();
        assert_eq!(s.pages[0][0], Some(AppItem::new("/A.app", "A")));
    }
    #[test] fn test_app_item_without_icon_omits_icon_key() {
        assert!(!serde_json::to_string(&AppItem::new("/A.app", "A")).unwrap().contains("icon"));
    }
    #[test] fn test_remove_app_returns_item_and_empties_slot() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 3, item("A")).unwrap();
        assert_eq!(s.remove_app(0, 3), Ok(item("A")));
        assert_eq!(s.pages[0][3], None);
    }
    #[test] fn test_remove_app_rejects_invalid_page() { let mut s = LauncherState::new(1); assert_eq!(s.remove_app(1, 0), Err("Invalid page index".to_string())); }
    #[test] fn test_remove_app_rejects_invalid_slot() { let mut s = LauncherState::new(1); assert_eq!(s.remove_app(0, 8), Err("Invalid slot index".to_string())); }
    #[test] fn test_remove_app_rejects_empty_slot() { let mut s = LauncherState::new(1); assert_eq!(s.remove_app(0, 0), Err("Slot is empty".to_string())); }
    #[test] fn test_rename_app_changes_only_the_name() {
        let mut s = LauncherState::new(1);
        let it = AppItem::url("https://calendar.google.com/", "calendar.google.com", Some("data:icon".into()));
        s.add_app(0, 2, it.clone()).unwrap();
        s.rename_app(0, 2, "Googleカレンダー").unwrap();
        assert_eq!(s.pages[0][2], Some(AppItem { name: "Googleカレンダー".into(), ..it }));
    }
    #[test] fn test_rename_app_trims_surrounding_whitespace() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 0, item("A")).unwrap();
        s.rename_app(0, 0, "  新しい名前 \n").unwrap();
        assert_eq!(s.pages[0][0].as_ref().unwrap().name, "新しい名前");
    }
    #[test] fn test_rename_app_rejects_a_blank_name() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 0, item("A")).unwrap();
        assert_eq!(s.rename_app(0, 0, "   "), Err("Name is empty".to_string()));
        assert_eq!(s.pages[0][0], Some(item("A")));
    }
    #[test] fn test_rename_app_rejects_a_too_long_name() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 0, item("A")).unwrap();
        assert!(s.rename_app(0, 0, &"あ".repeat(MAX_NAME_CHARS)).is_ok());
        assert_eq!(s.rename_app(0, 0, &"あ".repeat(MAX_NAME_CHARS + 1)), Err("Name is too long".to_string()));
    }
    #[test] fn test_rename_app_rejects_an_empty_slot_or_invalid_position() {
        let mut s = LauncherState::new(1);
        assert_eq!(s.rename_app(0, 0, "A"), Err("Slot is empty".to_string()));
        assert_eq!(s.rename_app(1, 0, "A"), Err("Invalid page index".to_string()));
        assert_eq!(s.rename_app(0, 8, "A"), Err("Invalid slot index".to_string()));
    }
    #[test] fn test_move_app_to_empty_slot_across_pages() {
        let mut s = LauncherState::new(2);
        s.add_app(0, 0, item("A")).unwrap();
        assert!(s.move_app((0, 0), (1, 5)).is_ok());
        assert_eq!(s.pages[0][0], None);
        assert_eq!(s.pages[1][5], Some(item("A")));
    }
    #[test] fn test_move_app_swaps_when_destination_occupied() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 0, item("A")).unwrap();
        s.add_app(0, 1, item("B")).unwrap();
        assert!(s.move_app((0, 0), (0, 1)).is_ok());
        assert_eq!(s.pages[0][0], Some(item("B")));
        assert_eq!(s.pages[0][1], Some(item("A")));
    }
    #[test] fn test_move_app_to_same_slot_is_noop() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 2, item("A")).unwrap();
        assert!(s.move_app((0, 2), (0, 2)).is_ok());
        assert_eq!(s.pages[0][2], Some(item("A")));
    }
    #[test] fn test_move_app_rejects_empty_source() { let mut s = LauncherState::new(1); assert_eq!(s.move_app((0, 0), (0, 1)), Err("Slot is empty".to_string())); }
    #[test] fn test_move_app_rejects_invalid_source() {
        let mut s = LauncherState::new(1);
        assert_eq!(s.move_app((1, 0), (0, 0)), Err("Invalid page index".to_string()));
        assert_eq!(s.move_app((0, 8), (0, 0)), Err("Invalid slot index".to_string()));
    }
    #[test] fn test_move_app_rejects_invalid_destination_without_losing_source() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 0, item("A")).unwrap();
        assert_eq!(s.move_app((0, 0), (1, 0)), Err("Invalid page index".to_string()));
        assert_eq!(s.move_app((0, 0), (0, 8)), Err("Invalid slot index".to_string()));
        assert_eq!(s.pages[0][0], Some(item("A")));
    }
    #[test] fn test_add_to_first_empty_rejects_full_page() {
        let mut s = LauncherState::new(1);
        for i in 0..8 { s.add_app_to_first_empty(0, item(&i.to_string())).unwrap(); }
        assert_eq!(s.add_app_to_first_empty(0, item("X")), Err("No empty slot on this page".to_string()));
    }
    #[test] fn test_insert_app_into_an_empty_slot_places_it_there() {
        let mut s = LauncherState::new(1);
        s.insert_app(0, 3, item("A")).unwrap();
        assert_eq!(s.pages[0][3], Some(item("A")));
    }
    #[test] fn test_insert_app_shifts_items_up_to_the_next_empty_slot() {
        let mut s = LauncherState::new(1);
        for (i, n) in ["A", "B", "C"].iter().enumerate() { s.add_app(0, i + 1, item(n)).unwrap(); }
        s.add_app(0, 6, item("Z")).unwrap();
        s.insert_app(0, 1, item("X")).unwrap();
        let names: Vec<_> = s.pages[0].iter().map(|x| x.as_ref().map(|a| a.name.as_str())).collect();
        assert_eq!(names, [None, Some("X"), Some("A"), Some("B"), Some("C"), None, Some("Z"), None]);
    }
    #[test] fn test_insert_app_rejects_when_no_empty_slot_follows_without_changing_anything() {
        let mut s = LauncherState::new(1);
        for i in 1..8 { s.add_app(0, i, item(&i.to_string())).unwrap(); }
        let before = s.pages.clone();
        assert_eq!(s.insert_app(0, 2, item("X")), Err("No empty slot after this position".to_string()));
        assert_eq!(s.pages, before);
    }
    #[test] fn test_insert_app_rejects_an_invalid_position() {
        let mut s = LauncherState::new(1);
        assert_eq!(s.insert_app(1, 0, item("X")), Err("Invalid page index".to_string()));
        assert_eq!(s.insert_app(0, 8, item("X")), Err("Invalid slot index".to_string()));
    }
}
