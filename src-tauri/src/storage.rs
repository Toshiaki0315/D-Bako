//! インフラ層: `LauncherState` の永続化。ファイル操作は `FileSystem` トレイト越しに行い、
//! テストではインメモリのモックに差し替える。

use crate::core::LauncherState;
use std::io;
use std::path::{Path, PathBuf};

pub trait FileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn write(&self, path: &Path, contents: &str) -> io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
}

/// 実ファイルシステム。
pub struct StdFs;

impl FileSystem for StdFs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> { std::fs::read_to_string(path) }
    fn write(&self, path: &Path, contents: &str) -> io::Result<()> { std::fs::write(path, contents) }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> { std::fs::rename(from, to) }
    fn create_dir_all(&self, path: &Path) -> io::Result<()> { std::fs::create_dir_all(path) }
}

pub struct StateStore<F: FileSystem> {
    fs: F,
    path: PathBuf,
}

impl<F: FileSystem> StateStore<F> {
    pub fn new(fs: F, path: PathBuf) -> Self { Self { fs, path } }

    /// 保存済みの状態を読み込む。ファイルが無ければ初期状態 (10 ページ) を返す。
    pub fn load(&self) -> Result<LauncherState, String> {
        match self.fs.read_to_string(&self.path) {
            Ok(json) => LauncherState::from_json(&json),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(LauncherState::new(10)),
            Err(e) => Err(e.to_string()),
        }
    }

    /// 一時ファイルへ書いてから rename することで、書き込み途中の破損を避ける。
    pub fn save(&self, state: &LauncherState) -> Result<(), String> {
        let json = state.to_json()?;
        if let Some(dir) = self.path.parent() {
            self.fs.create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = self.path.with_extension("json.tmp");
        self.fs.write(&tmp, &json).map_err(|e| e.to_string())?;
        self.fs.rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::AppItem;
    use std::cell::RefCell;
    use std::collections::{HashMap, HashSet};
    use std::io::{Error, ErrorKind};

    /// インメモリのファイルシステム。`fail_on` に操作名を入れるとその操作が失敗する。
    #[derive(Default)]
    struct MockFs {
        files: RefCell<HashMap<PathBuf, String>>,
        dirs: RefCell<HashSet<PathBuf>>,
        fail_on: Option<&'static str>,
    }

    impl MockFs {
        fn failing(op: &'static str) -> Self { Self { fail_on: Some(op), ..Default::default() } }
        fn check(&self, op: &str) -> io::Result<()> {
            if self.fail_on == Some(op) { Err(Error::new(ErrorKind::PermissionDenied, format!("{op} denied"))) } else { Ok(()) }
        }
    }

    impl FileSystem for MockFs {
        fn read_to_string(&self, path: &Path) -> io::Result<String> {
            self.check("read")?;
            self.files.borrow().get(path).cloned().ok_or_else(|| Error::from(ErrorKind::NotFound))
        }
        fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
            self.check("write")?;
            self.files.borrow_mut().insert(path.to_path_buf(), contents.to_string());
            Ok(())
        }
        fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
            self.check("rename")?;
            let data = self.files.borrow_mut().remove(from).ok_or_else(|| Error::from(ErrorKind::NotFound))?;
            self.files.borrow_mut().insert(to.to_path_buf(), data);
            Ok(())
        }
        fn create_dir_all(&self, path: &Path) -> io::Result<()> {
            self.check("mkdir")?;
            self.dirs.borrow_mut().insert(path.to_path_buf());
            Ok(())
        }
    }

    fn store(fs: MockFs) -> StateStore<MockFs> { StateStore::new(fs, PathBuf::from("/data/dbako/state.json")) }

    #[test]
    fn load_returns_default_state_when_file_is_missing() {
        let state = store(MockFs::default()).load().unwrap();
        assert_eq!(state.max_pages, 10);
        assert!(state.pages.iter().flatten().all(|s| s.is_none()));
    }

    #[test]
    fn save_then_load_restores_state() {
        let s = store(MockFs::default());
        let mut state = LauncherState::new(3);
        state.add_app(1, 2, AppItem { path: "/A.app".into(), name: "A".into() }).unwrap();
        s.save(&state).unwrap();
        let restored = s.load().unwrap();
        assert_eq!(restored.max_pages, 3);
        assert_eq!(restored.pages, state.pages);
    }

    #[test]
    fn save_creates_parent_dir_and_leaves_no_temp_file() {
        let s = store(MockFs::default());
        s.save(&LauncherState::new(1)).unwrap();
        assert!(s.fs.dirs.borrow().contains(Path::new("/data/dbako")));
        let files = s.fs.files.borrow();
        assert_eq!(files.keys().collect::<Vec<_>>(), vec![&PathBuf::from("/data/dbako/state.json")]);
    }

    #[test]
    fn load_errors_on_corrupt_file() {
        let fs = MockFs::default();
        fs.files.borrow_mut().insert(PathBuf::from("/data/dbako/state.json"), "{broken".into());
        assert!(store(fs).load().is_err());
    }

    #[test]
    fn load_errors_on_invalid_state_content() {
        let fs = MockFs::default();
        fs.files.borrow_mut().insert(PathBuf::from("/data/dbako/state.json"), r#"{"pages":[],"max_pages":0}"#.into());
        assert_eq!(store(fs).load().unwrap_err(), "Invalid max_pages");
    }

    #[test]
    fn load_propagates_non_notfound_io_errors() {
        assert!(store(MockFs::failing("read")).load().unwrap_err().contains("read denied"));
    }

    #[test]
    fn save_propagates_each_io_failure() {
        for op in ["mkdir", "write", "rename"] {
            let err = store(MockFs::failing(op)).save(&LauncherState::new(1)).unwrap_err();
            assert!(err.contains(&format!("{op} denied")), "{op}: {err}");
        }
    }

    #[test]
    fn std_fs_roundtrips_in_nested_directory() {
        let dir = tempfile::tempdir().unwrap();
        let s = StateStore::new(StdFs, dir.path().join("a/b/state.json"));
        let mut state = LauncherState::new(2);
        state.add_app(0, 0, AppItem { path: "/A.app".into(), name: "A".into() }).unwrap();
        s.save(&state).unwrap();
        assert_eq!(s.load().unwrap().pages, state.pages);
        assert!(!dir.path().join("a/b/state.json.tmp").exists());
    }

    #[test]
    fn std_fs_load_returns_default_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let s = StateStore::new(StdFs, dir.path().join("none.json"));
        assert_eq!(s.load().unwrap().max_pages, 10);
    }
}
