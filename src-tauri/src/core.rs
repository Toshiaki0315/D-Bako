use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppItem { pub path: String, pub name: String }
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
    #[test] fn test_add_app() { let mut state = LauncherState::new(1); let app = AppItem { path: "/A.app".into(), name: "A".into() }; assert!(state.add_app(0, 0, app.clone()).is_ok()); assert_eq!(state.pages[0][0], Some(app)); }
    fn item(n: &str) -> AppItem { AppItem { path: format!("/{n}.app"), name: n.into() } }
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
    #[test] fn test_remove_app_returns_item_and_empties_slot() {
        let mut s = LauncherState::new(1);
        s.add_app(0, 3, item("A")).unwrap();
        assert_eq!(s.remove_app(0, 3), Ok(item("A")));
        assert_eq!(s.pages[0][3], None);
    }
    #[test] fn test_remove_app_rejects_invalid_page() { let mut s = LauncherState::new(1); assert_eq!(s.remove_app(1, 0), Err("Invalid page index".to_string())); }
    #[test] fn test_remove_app_rejects_invalid_slot() { let mut s = LauncherState::new(1); assert_eq!(s.remove_app(0, 8), Err("Invalid slot index".to_string())); }
    #[test] fn test_remove_app_rejects_empty_slot() { let mut s = LauncherState::new(1); assert_eq!(s.remove_app(0, 0), Err("Slot is empty".to_string())); }
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
}
