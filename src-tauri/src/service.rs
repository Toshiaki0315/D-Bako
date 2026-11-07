//! アプリケーション層: 状態の変更と永続化を束ねる。Tauri には依存しない。

use crate::core::{AppItem, LauncherState};
use crate::storage::{FileSystem, LoadError, StateStore};

pub struct LauncherService<F: FileSystem> {
    state: LauncherState,
    store: StateStore<F>,
    /// 保存済みの状態を読めず、ファイルをそのままにして起動したときの理由。ある間は保存しない。
    load_problem: Option<String>,
}

impl<F: FileSystem> LauncherService<F> {
    /// 保存済みの状態で起動する。
    /// - ファイルが無ければ、初期状態で始める。
    /// - 中身が壊れていれば、`.bak` に退避して初期状態で始める。
    /// - 読み出せなかった (権限エラーなど) 場合は、ファイルに触れず初期状態で始め、変更の保存は断る。
    ///   (読めなかっただけの無傷のファイルを、空に近い状態で上書きして失わないため)
    pub fn open(store: StateStore<F>) -> Self {
        let (state, load_problem) = match store.load() {
            Ok(state) => (state, None),
            Err(LoadError::Corrupt(_)) => match store.backup_corrupt() {
                Ok(()) => (LauncherState::new(10), None),
                Err(e) => (LauncherState::new(10), Some(format!("the corrupt state file could not be backed up: {e}"))),
            },
            Err(e @ LoadError::Io(_)) => (LauncherState::new(10), Some(e.to_string())),
        };
        Self { state, store, load_problem }
    }

    pub fn state(&self) -> &LauncherState { &self.state }

    /// 保存済みの状態を読めなかったために、変更を保存できない状態のときの理由。
    pub fn load_problem(&self) -> Option<&str> { self.load_problem.as_deref() }

    /// 変更して保存する。失敗した場合はメモリ上の状態も元に戻す。
    fn mutate(&mut self, f: impl FnOnce(&mut LauncherState) -> Result<(), String>) -> Result<(), String> {
        if let Some(reason) = &self.load_problem {
            return Err(format!("Saved state could not be read ({reason}); changes are not saved"));
        }
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

    /// グリッドの横・縦の数とページ数を変える。収まらなくなったアイテムは空きスロットへ移す。
    pub fn set_layout(&mut self, cols: usize, rows: usize, max_pages: usize) -> Result<(), String> {
        self.set_layout_then(cols, rows, max_pages, || Ok(()))
    }

    /// `set_layout` のあとに続く処理 (ウィンドウの大きさの変更など) まで成功して初めて確定する。
    /// 続きの処理が失敗したら、メモリ上の状態と保存ファイルの両方を元の設定に戻す。
    pub fn set_layout_then(
        &mut self,
        cols: usize,
        rows: usize,
        max_pages: usize,
        apply: impl FnOnce() -> Result<(), String>,
    ) -> Result<(), String> {
        let before = self.state.clone();
        self.mutate(|s| s.set_layout(cols, rows, max_pages))?;
        if let Err(e) = apply() {
            self.state = before;
            return Err(match self.store.save(&self.state) {
                Ok(()) => e,
                Err(restore) => format!("{e} (also failed to restore the saved state: {restore})"),
            });
        }
        Ok(())
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
    use std::io;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    /// 1 つの `MockFs` をサービスとテストの両方から触れるようにする (途中で失敗を仕込むため)。
    struct SharedFs(Rc<MockFs>);

    impl FileSystem for SharedFs {
        fn read_to_string(&self, path: &Path) -> io::Result<String> { self.0.read_to_string(path) }
        fn write(&self, path: &Path, contents: &str) -> io::Result<()> { self.0.write(path, contents) }
        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> { self.0.rename(from, to) }
        fn create_dir_all(&self, path: &Path) -> io::Result<()> { self.0.create_dir_all(path) }
    }

    fn layout(s: &LauncherState) -> (usize, usize, usize) { (s.cols, s.rows, s.max_pages) }

    /// 読み込みだけが失敗するファイルシステムに、有効な保存データを 1 件置く。
    fn unreadable_saved_state() -> (MockFs, String) {
        let fs = MockFs::failing("read");
        let mut saved = LauncherState::new(4);
        saved.add_app(0, 0, item("Precious")).unwrap();
        let json = saved.to_json().unwrap();
        fs.files.borrow_mut().insert(PathBuf::from(STATE_PATH), json.clone());
        (fs, json)
    }

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
    fn open_backs_up_a_file_whose_content_fails_validation() {
        let fs = MockFs::default();
        let invalid = r#"{"pages":[],"max_pages":0}"#;
        fs.files.borrow_mut().insert(PathBuf::from(STATE_PATH), invalid.into());

        let svc = service(fs);
        assert_eq!(svc.state().max_pages, 10);
        assert_eq!(svc.store.fs.files.borrow().get(Path::new("/data/dbako/state.json.bak")).map(String::as_str), Some(invalid));
        assert_eq!(svc.load_problem(), None);
    }

    #[test]
    fn open_does_not_move_a_file_that_could_not_be_read_for_io_reasons() {
        let (fs, json) = unreadable_saved_state();
        let svc = service(fs);

        assert_eq!(svc.state().max_pages, 10);
        assert!(svc.load_problem().unwrap().contains("read denied"));
        let files = svc.store.fs.files.borrow();
        assert_eq!(files.get(Path::new(STATE_PATH)), Some(&json));
        assert!(!files.contains_key(Path::new("/data/dbako/state.json.bak")));
    }

    #[test]
    fn changes_are_refused_instead_of_overwriting_a_file_that_could_not_be_read() {
        let (fs, json) = unreadable_saved_state();
        let mut svc = service(fs);

        let err = svc.add_app_to_first_empty(0, item("New")).unwrap_err();
        assert!(err.contains("read denied") && err.contains("changes are not saved"), "{err}");
        assert_eq!(svc.state().pages[0][0], None);
        assert_eq!(svc.store.fs.files.borrow().get(Path::new(STATE_PATH)), Some(&json));
    }

    #[test]
    fn a_corrupt_file_that_cannot_be_backed_up_is_also_left_alone() {
        let fs = MockFs::failing("rename");
        fs.files.borrow_mut().insert(PathBuf::from(STATE_PATH), "{broken".into());
        let mut svc = service(fs);

        assert!(svc.load_problem().unwrap().contains("rename denied"));
        assert!(svc.add_app_to_first_empty(0, item("A")).is_err());
        assert_eq!(svc.store.fs.files.borrow().get(Path::new(STATE_PATH)).map(String::as_str), Some("{broken"));
    }

    #[test]
    fn a_missing_file_is_not_a_problem() {
        let mut svc = service(MockFs::default());
        assert_eq!(svc.load_problem(), None);
        assert!(svc.add_app_to_first_empty(0, item("A")).is_ok());
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
    fn set_layout_then_commits_when_the_follow_up_succeeds() {
        let mut svc = service(MockFs::default());
        let mut called = false;
        svc.set_layout_then(3, 2, 4, || { called = true; Ok(()) }).unwrap();

        assert!(called);
        assert_eq!(layout(svc.state()), (3, 2, 4));
        assert_eq!(layout(&svc.store.load().unwrap()), (3, 2, 4));
    }

    #[test]
    fn set_layout_then_rolls_back_memory_and_file_when_the_follow_up_fails() {
        let mut svc = service(MockFs::default());
        svc.set_layout(4, 2, 3).unwrap();

        let err = svc.set_layout_then(5, 3, 2, || Err("window failed".to_string())).unwrap_err();

        assert_eq!(err, "window failed");
        assert_eq!(layout(svc.state()), (4, 2, 3));
        assert_eq!(layout(&svc.store.load().unwrap()), (4, 2, 3));
    }

    #[test]
    fn set_layout_then_skips_the_follow_up_when_the_change_is_rejected() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_first_empty(0, item("A")).unwrap();
        svc.add_app_to_first_empty(0, item("B")).unwrap();

        let mut called = false;
        let result = svc.set_layout_then(1, 1, 1, || { called = true; Ok(()) });

        assert!(result.is_err());
        assert!(!called);
        assert_eq!(layout(svc.state()), (4, 2, 10));
    }

    #[test]
    fn set_layout_then_reports_when_the_previous_state_cannot_be_restored() {
        let fs = Rc::new(MockFs::default());
        let mut svc = LauncherService::open(StateStore::new(SharedFs(fs.clone()), PathBuf::from(STATE_PATH)));
        svc.set_layout(4, 2, 3).unwrap();

        let err = svc
            .set_layout_then(5, 3, 2, move || { fs.fail_on.set(Some("write")); Err("window failed".to_string()) })
            .unwrap_err();

        assert!(err.contains("window failed") && err.contains("restore"), "{err}");
        assert_eq!(layout(svc.state()), (4, 2, 3));
    }

    #[test]
    fn set_layout_changes_and_persists_the_grid() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 7, item("A")).unwrap();
        svc.set_layout(3, 2, 4).unwrap();

        let saved = svc.store.load().unwrap();
        assert_eq!((saved.cols, saved.rows, saved.max_pages), (3, 2, 4));
        assert_eq!(saved.pages[0][0], Some(item("A")));
    }

    #[test]
    fn set_layout_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::default());
        svc.add_app_to_slot(0, 7, item("A")).unwrap();
        svc.store.fs.fail_on.set(Some("write"));

        assert!(svc.set_layout(3, 2, 4).unwrap_err().contains("write denied"));
        assert_eq!((svc.state().cols, svc.state().rows, svc.state().max_pages), (4, 2, 10));
        assert_eq!(svc.state().pages[0][7], Some(item("A")));
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
