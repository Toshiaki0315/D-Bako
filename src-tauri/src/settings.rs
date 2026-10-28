//! アプリ設定 (「常に前に表示」など) と、その永続化・変更を束ねるアプリケーション層。Tauri には依存しない。

use crate::storage::{write_atomic, FileSystem};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 項目を増やしても古い保存ファイルを読めるよう、欠けた項目は既定値で補う。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct Settings {
    pub always_on_top: bool,
}

pub struct SettingsService<F: FileSystem> {
    settings: Settings,
    fs: F,
    path: PathBuf,
}

impl<F: FileSystem> SettingsService<F> {
    /// 保存済みの設定で起動する。無い・読めない場合は既定値 (設定は失っても困らないので退避はしない)。
    pub fn open(fs: F, path: PathBuf) -> Self {
        let settings = fs.read_to_string(&path).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default();
        Self { settings, fs, path }
    }

    pub fn settings(&self) -> &Settings { &self.settings }

    /// 保存できた場合だけメモリ上の設定を変える (失敗してもファイルとずれない)。
    pub fn set_always_on_top(&mut self, enabled: bool) -> Result<(), String> {
        let mut next = self.settings.clone();
        next.always_on_top = enabled;
        let json = serde_json::to_string(&next).map_err(|e| e.to_string())?;
        write_atomic(&self.fs, &self.path, &json)?;
        self.settings = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mock::MockFs;
    use std::path::Path;

    const PATH: &str = "/data/dbako/settings.json";

    fn service(fs: MockFs) -> SettingsService<MockFs> { SettingsService::open(fs, PathBuf::from(PATH)) }
    fn with_file(contents: &str) -> MockFs {
        let fs = MockFs::default();
        fs.files.borrow_mut().insert(PathBuf::from(PATH), contents.into());
        fs
    }

    #[test]
    fn defaults_to_not_always_on_top() {
        assert!(!Settings::default().always_on_top);
    }

    #[test]
    fn open_uses_defaults_when_nothing_is_saved() {
        assert_eq!(service(MockFs::default()).settings(), &Settings::default());
    }

    #[test]
    fn open_restores_saved_settings() {
        assert!(service(with_file(r#"{"always_on_top":true}"#)).settings().always_on_top);
    }

    #[test]
    fn open_fills_missing_fields_with_defaults() {
        assert_eq!(service(with_file("{}")).settings(), &Settings::default());
    }

    #[test]
    fn open_falls_back_to_defaults_on_a_corrupt_file_or_read_error() {
        assert_eq!(service(with_file("{broken")).settings(), &Settings::default());
        assert_eq!(service(MockFs::failing("read")).settings(), &Settings::default());
    }

    #[test]
    fn set_always_on_top_updates_and_persists() {
        let mut svc = service(MockFs::default());
        svc.set_always_on_top(true).unwrap();
        assert!(svc.settings().always_on_top);

        let saved = svc.fs.files.borrow().get(Path::new(PATH)).cloned().unwrap();
        let reopened = SettingsService::open(with_file(&saved), PathBuf::from(PATH));
        assert!(reopened.settings().always_on_top);
    }

    #[test]
    fn set_always_on_top_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::failing("write"));
        assert!(svc.set_always_on_top(true).unwrap_err().contains("write denied"));
        assert!(!svc.settings().always_on_top);
    }
}
