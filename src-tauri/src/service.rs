//! アプリケーション層: 状態の変更と永続化を束ねる。Tauri には依存しない。

use crate::core::{AppItem, LauncherState};
use crate::storage::{FileSystem, StateStore};

pub struct LauncherService<F: FileSystem> {
    state: LauncherState,
    store: StateStore<F>,
}

impl<F: FileSystem> LauncherService<F> {
    /// 保存済みの状態で起動する。読み込めなければ壊れたファイルを `.bak` に退避し、初期状態で始める。
    pub fn open(store: StateStore<F>) -> Self {
        let state = store.load().unwrap_or_else(|_| {
            let _ = store.backup_corrupt();
            LauncherState::new(10)
        });
        Self { state, store }
    }

    pub fn state(&self) -> &LauncherState { &self.state }

    /// 変更して保存する。失敗した場合はメモリ上の状態も元に戻す。
    fn mutate(&mut self, f: impl FnOnce(&mut LauncherState) -> Result<(), String>) -> Result<(), String> {
        let before = self.state.clone();
        f(&mut self.state)?;
        if let Err(e) = self.store.save(&self.state) {
            self.state = before;
            return Err(e);
        }
        Ok(())
    }

    pub fn add_app_to_slot(&mut self, page_idx: usize, slot_idx: usize, app: AppItem) -> Result<(), String> {
        self.mutate(|s| s.add_app_to_empty_slot(page_idx, slot_idx, app))
    }

    pub fn add_app_to_first_empty(&mut self, page_idx: usize, app: AppItem) -> Result<(), String> {
        self.mutate(|s| s.add_app_to_first_empty(page_idx, app))
    }

    pub fn rename_app(&mut self, page_idx: usize, slot_idx: usize, name: &str) -> Result<(), String> {
        self.mutate(|s| s.rename_app(page_idx, slot_idx, name))
    }

    /// 登録を解除してスロットを空ける。アプリ本体には触れない。
    pub fn remove_app(&mut self, page_idx: usize, slot_idx: usize) -> Result<(), String> {
        self.mutate(|s| s.remove_app(page_idx, slot_idx).map(|_| ()))
    }

    /// 同じページ内で移動する。移動先が埋まっていれば入れ替える。
    pub fn move_app(&mut self, page_idx: usize, from_slot: usize, to_slot: usize) -> Result<(), String> {
        self.mutate(|s| s.move_app((page_idx, from_slot), (page_idx, to_slot)))
    }

    /// 指定スロットに差し込み、埋まっていれば後ろへずらす。
    pub fn insert_app(&mut self, page_idx: usize, slot_idx: usize, app: AppItem) -> Result<(), String> {
        self.mutate(|s| s.insert_app(page_idx, slot_idx, app))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mock::MockFs;
    use std::path::{Path, PathBuf};

    const STATE_PATH: &str = "/data/dbako/state.json";

    fn item(n: &str) -> AppItem { AppItem::new(format!("/{n}.app"), n) }
    fn service(fs: MockFs) -> LauncherService<MockFs> {
        LauncherService::open(StateStore::new(fs, PathBuf::from(STATE_PATH)))
    }

    #[test]
    fn open_starts_with_default_state_when_nothing_is_saved() {
        let svc = service(MockFs::default());
        assert_eq!(svc.state().max_pages, 10);
        assert!(svc.state().pages.iter().flatten().all(|s| s.is_none()));
    }

    #[test]
    fn open_restores_the_saved_state() {
        let fs = MockFs::default();
        let mut saved = LauncherState::new(4);
        saved.add_app(3, 1, item("A")).unwrap();
        fs.files.borrow_mut().insert(PathBuf::from(STATE_PATH), saved.to_json().unwrap());

        let svc = service(fs);
        assert_eq!(svc.state().max_pages, 4);
        assert_eq!(svc.state().pages[3][1], Some(item("A")));
    }

    #[test]
    fn open_backs_up_a_corrupt_file_and_starts_fresh() {
        let fs = MockFs::default();
        fs.files.borrow_mut().insert(PathBuf::from(STATE_PATH), "{broken".into());

        let svc = service(fs);
        assert_eq!(svc.state().max_pages, 10);
        let files = svc.store.fs.files.borrow();
        assert_eq!(files.get(Path::new("/data/dbako/state.json.bak")).map(String::as_str), Some("{broken"));
        assert!(!files.contains_key(Path::new(STATE_PATH)));
    }

    #[test]
    fn open_survives_when_the_backup_itself_fails() {
        let fs = MockFs::failing("rename");
        fs.files.borrow_mut().insert(PathBuf::from(STATE_PATH), "{broken".into());
        assert_eq!(service(fs).state().max_pages, 10);
    }

    #[test]
    fn add_app_updates_state_and_persists_it() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_first_empty(1, item("A")).unwrap();

        assert_eq!(svc.state().pages[1][0], Some(item("A")));
        assert_eq!(svc.store.load().unwrap().pages[1][0], Some(item("A")));
    }

    #[test]
    fn add_app_to_slot_persists_and_refuses_an_occupied_slot() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 3, item("A")).unwrap();
        assert_eq!(svc.store.load().unwrap().pages[0][3], Some(item("A")));

        assert_eq!(svc.add_app_to_slot(0, 3, item("B")), Err("Slot is occupied".to_string()));
        assert_eq!(svc.state().pages[0][3], Some(item("A")));
    }

    #[test]
    fn add_app_to_slot_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::failing("write"));
        assert!(svc.add_app_to_slot(0, 0, item("A")).is_err());
        assert_eq!(svc.state().pages[0][0], None);
    }

    #[test]
    fn rename_app_updates_and_persists_the_name() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 1, item("A")).unwrap();
        svc.rename_app(0, 1, "エー").unwrap();

        assert_eq!(svc.state().pages[0][1].as_ref().unwrap().name, "エー");
        assert_eq!(svc.store.load().unwrap().pages[0][1].as_ref().unwrap().name, "エー");
    }

    #[test]
    fn rename_app_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 1, item("A")).unwrap();
        svc.store.fs.fail_on.set(Some("write"));

        assert!(svc.rename_app(0, 1, "エー").unwrap_err().contains("write denied"));
        assert_eq!(svc.state().pages[0][1], Some(item("A")));
    }

    #[test]
    fn remove_app_empties_the_slot_and_persists_it() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 1, item("A")).unwrap();
        svc.remove_app(0, 1).unwrap();

        assert_eq!(svc.state().pages[0][1], None);
        assert_eq!(svc.store.load().unwrap().pages[0][1], None);
    }

    #[test]
    fn remove_app_rejects_an_empty_slot() {
        let mut svc = service(MockFs::default());
        assert_eq!(svc.remove_app(0, 1), Err("Slot is empty".to_string()));
    }

    #[test]
    fn remove_app_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 1, item("A")).unwrap();
        svc.store.fs.fail_on.set(Some("write"));

        assert!(svc.remove_app(0, 1).unwrap_err().contains("write denied"));
        assert_eq!(svc.state().pages[0][1], Some(item("A")));
    }

    #[test]
    fn move_app_swaps_and_persists() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 0, item("A")).unwrap();
        svc.add_app_to_slot(0, 3, item("B")).unwrap();
        svc.move_app(0, 0, 3).unwrap();

        let saved = svc.store.load().unwrap();
        assert_eq!((saved.pages[0][0].clone(), saved.pages[0][3].clone()), (Some(item("B")), Some(item("A"))));
    }

    #[test]
    fn move_app_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 0, item("A")).unwrap();
        svc.store.fs.fail_on.set(Some("write"));

        assert!(svc.move_app(0, 0, 3).unwrap_err().contains("write denied"));
        assert_eq!(svc.state().pages[0][0], Some(item("A")));
        assert_eq!(svc.state().pages[0][3], None);
    }

    #[test]
    fn insert_app_shifts_and_persists() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 0, item("A")).unwrap();
        svc.insert_app(0, 0, item("X")).unwrap();

        let saved = svc.store.load().unwrap();
        assert_eq!((saved.pages[0][0].clone(), saved.pages[0][1].clone()), (Some(item("X")), Some(item("A"))));
    }

    #[test]
    fn insert_app_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 0, item("A")).unwrap();
        svc.store.fs.fail_on.set(Some("write"));

        assert!(svc.insert_app(0, 0, item("X")).unwrap_err().contains("write denied"));
        assert_eq!(svc.state().pages[0][0], Some(item("A")));
        assert_eq!(svc.state().pages[0][1], None);
    }

    #[test]
    fn add_app_with_invalid_page_errors_and_writes_nothing() {
        let mut svc = service(MockFs::default());
        assert_eq!(svc.add_app_to_first_empty(99, item("A")), Err("Invalid page index".to_string()));
        assert!(svc.store.fs.files.borrow().is_empty());
    }

    #[test]
    fn add_app_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::default());
        svc.store.fs.fail_on.set(Some("write"));

        assert!(svc.add_app_to_first_empty(0, item("A")).unwrap_err().contains("write denied"));
        assert_eq!(svc.state().pages[0][0], None);

        svc.store.fs.fail_on.set(None);
        svc.add_app_to_first_empty(0, item("A")).unwrap();
        assert_eq!(svc.state().pages[0][0], Some(item("A")));
    }
}
