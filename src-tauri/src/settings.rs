//! アプリ設定 (「常に前に表示」など) と、その永続化・変更を束ねるアプリケーション層。Tauri には依存しない。

use crate::storage::{write_atomic, FileSystem};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 項目を増やしても古い保存ファイルを読めるよう、欠けた項目は既定値で補う。
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub always_on_top: bool,
    /// 前回終了時のウィンドウ幅 (論理ピクセル)。高さは縦横比から決まるので保存しない。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_width: Option<f64>,
}

pub struct SettingsService<F: FileSystem> {
    settings: Settings,
    /// 覚えただけでまだ保存していない変更があるか。
    dirty: bool,
    fs: F,
    path: PathBuf,
}

impl<F: FileSystem> SettingsService<F> {
    /// 保存済みの設定で起動する。無い・読めない場合は既定値 (設定は失っても困らないので退避はしない)。
    pub fn open(fs: F, path: PathBuf) -> Self {
        let settings = fs.read_to_string(&path).ok().and_then(|json| serde_json::from_str(&json).ok()).unwrap_or_default();
        Self { settings, dirty: false, fs, path }
    }

    pub fn settings(&self) -> &Settings { &self.settings }

    fn write(&self, settings: &Settings) -> Result<(), String> {
        let json = serde_json::to_string(settings).map_err(|e| e.to_string())?;
        write_atomic(&self.fs, &self.path, &json)
    }

    /// 保存できた場合だけメモリ上の設定を変える (失敗してもファイルとずれない)。
    pub fn set_always_on_top(&mut self, enabled: bool) -> Result<(), String> {
        let mut next = self.settings.clone();
        next.always_on_top = enabled;
        self.write(&next)?;
        self.settings = next;
        self.dirty = false;
        Ok(())
    }

    /// ウィンドウ幅を覚える。リサイズ中は頻繁に呼ばれるので、保存は `flush` でまとめて行う。
    pub fn remember_window_width(&mut self, width: f64) {
        if self.settings.window_width != Some(width) {
            self.settings.window_width = Some(width);
            self.dirty = true;
        }
    }

    /// 覚えた変更があれば保存する。失敗した場合は次の `flush` で再び保存を試みる。
    pub fn flush(&mut self) -> Result<(), String> {
        if !self.dirty { return Ok(()); }
        self.write(&self.settings)?;
        self.dirty = false;
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

    fn saved(svc: &SettingsService<MockFs>) -> Option<String> { svc.fs.files.borrow().get(Path::new(PATH)).cloned() }

    #[test]
    fn open_restores_the_saved_window_width() {
        let svc = service(with_file(r#"{"always_on_top":false,"window_width":800.5}"#));
        assert_eq!(svc.settings().window_width, Some(800.5));
    }

    #[test]
    fn remember_window_width_does_not_write_until_flushed() {
        let mut svc = service(MockFs::default());
        svc.remember_window_width(800.0);
        svc.remember_window_width(820.0);
        assert_eq!(svc.settings().window_width, Some(820.0));
        assert_eq!(saved(&svc), None);

        svc.flush().unwrap();
        let reopened = SettingsService::open(with_file(&saved(&svc).unwrap()), PathBuf::from(PATH));
        assert_eq!(reopened.settings().window_width, Some(820.0));
    }

    #[test]
    fn flush_writes_nothing_without_changes() {
        let mut svc = service(MockFs::default());
        svc.flush().unwrap();
        assert_eq!(saved(&svc), None);

        svc.remember_window_width(800.0);
        svc.flush().unwrap();
        svc.fs.files.borrow_mut().clear();
        svc.flush().unwrap();
        assert_eq!(saved(&svc), None);
    }

    #[test]
    fn remembering_the_same_width_is_not_a_change() {
        let mut svc = service(with_file(r#"{"window_width":800.0}"#));
        svc.fs.files.borrow_mut().clear();
        svc.remember_window_width(800.0);
        svc.flush().unwrap();
        assert_eq!(saved(&svc), None);
    }

    #[test]
    fn flush_keeps_the_change_pending_when_saving_fails() {
        let mut svc = service(MockFs::failing("write"));
        svc.remember_window_width(800.0);
        assert!(svc.flush().unwrap_err().contains("write denied"));

        svc.fs.fail_on.set(None);
        svc.flush().unwrap();
        assert!(saved(&svc).unwrap().contains("800"));
    }

    #[test]
    fn saving_always_on_top_also_saves_the_remembered_width() {
        let mut svc = service(MockFs::default());
        svc.remember_window_width(800.0);
        svc.set_always_on_top(true).unwrap();
        svc.fs.files.borrow_mut().clear();
        svc.flush().unwrap();
        assert_eq!(saved(&svc), None, "already saved");
    }

    #[test]
    fn set_always_on_top_rolls_back_when_saving_fails() {
        let mut svc = service(MockFs::failing("write"));
        assert!(svc.set_always_on_top(true).unwrap_err().contains("write denied"));
        assert!(!svc.settings().always_on_top);
    }
}
