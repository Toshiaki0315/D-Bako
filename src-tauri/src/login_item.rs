//! 「起動時に実行する」(macOS のログイン項目) の判断ロジック。OS の API はトレイト越しに使い、テストではモックに差し替える。

use crate::storage::FileSystem;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginItemStatus {
    NotRegistered,
    Enabled,
    /// 登録済みだが、システム設定の「ログイン項目」でユーザーの許可が必要。
    RequiresApproval,
    NotFound,
}

/// OS のログイン項目 (SMAppService) への登録・解除。
pub trait LoginItem {
    fn status(&self) -> LoginItemStatus;
    fn register(&self) -> Result<(), String>;
    fn unregister(&self) -> Result<(), String>;
}

pub const NOT_IN_BUNDLE_MESSAGE: &str = "開発版では登録できません。.app から起動して設定してください";
pub const APPROVAL_MESSAGE: &str = "システム設定の「ログイン項目」で道具箱を許可してください";

/// 実行ファイルが入っている `.app` バンドル (`X.app/Contents/MacOS/exe`)。`.app` の外 (開発用バイナリ) なら None。
pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let is_app = bundle.extension().is_some_and(|ext| ext == "app");
    (macos.file_name()? == "MacOS" && contents.file_name()? == "Contents" && is_app).then(|| bundle.to_path_buf())
}

/// 設定画面に見せる状態。許可待ちも「登録した」ことには変わりないのでオンとして見せる。
pub fn is_enabled(item: &impl LoginItem) -> bool {
    matches!(item.status(), LoginItemStatus::Enabled | LoginItemStatus::RequiresApproval)
}

/// ログイン時に起動するよう登録・解除する。ログイン項目は `.app` 単位なので、`.app` の外からは登録しない。
pub fn set_enabled(item: &impl LoginItem, enabled: bool, in_bundle: bool) -> Result<(), String> {
    if !enabled {
        return if is_enabled(item) { item.unregister() } else { Ok(()) };
    }
    if item.status() == LoginItemStatus::Enabled { return Ok(()); }
    if !in_bundle { return Err(NOT_IN_BUNDLE_MESSAGE.to_string()); }
    item.register()?;
    match item.status() {
        LoginItemStatus::RequiresApproval => Err(APPROVAL_MESSAGE.to_string()),
        _ => Ok(()),
    }
}

/// 以前の版が `~/Library/LaunchAgents` に置いた登録ファイルを消す (開発用バイナリを起動してしまう原因になっていた)。
/// 道具箱のものと分かるファイルだけを消し、消したら true。
pub fn remove_legacy_launch_agent(fs: &impl FileSystem, path: &Path) -> Result<bool, String> {
    let plist = match fs.read_to_string(path) {
        Ok(plist) => plist,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e.to_string()),
    };
    if !plist.contains("<string>D-Bako</string>") { return Ok(false); }
    fs.remove_file(path).map_err(|e| e.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mock::MockFs;
    use std::cell::{Cell, RefCell};

    struct MockItem {
        status: Cell<LoginItemStatus>,
        /// register 後に移る状態 (許可待ちの再現用)。
        after_register: LoginItemStatus,
        fail: Option<&'static str>,
        calls: RefCell<Vec<&'static str>>,
    }

    impl MockItem {
        fn new(status: LoginItemStatus) -> Self {
            Self { status: Cell::new(status), after_register: LoginItemStatus::Enabled, fail: None, calls: RefCell::default() }
        }
    }

    impl LoginItem for MockItem {
        fn status(&self) -> LoginItemStatus { self.status.get() }
        fn register(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("register");
            if let Some(e) = self.fail { return Err(e.into()); }
            self.status.set(self.after_register);
            Ok(())
        }
        fn unregister(&self) -> Result<(), String> {
            self.calls.borrow_mut().push("unregister");
            if let Some(e) = self.fail { return Err(e.into()); }
            self.status.set(LoginItemStatus::NotRegistered);
            Ok(())
        }
    }

    use LoginItemStatus::*;

    #[test]
    fn bundle_of_finds_the_enclosing_app() {
        assert_eq!(bundle_of(Path::new("/Applications/D-Bako.app/Contents/MacOS/d-bako")), Some(PathBuf::from("/Applications/D-Bako.app")));
    }

    #[test]
    fn bundle_of_rejects_a_bare_executable() {
        assert_eq!(bundle_of(Path::new("/Users/me/D-Bako/src-tauri/target/debug/d-bako")), None);
        assert_eq!(bundle_of(Path::new("/Applications/D-Bako.app/d-bako")), None);
        assert_eq!(bundle_of(Path::new("d-bako")), None);
    }

    #[test]
    fn is_enabled_treats_registered_states_as_on() {
        assert!(is_enabled(&MockItem::new(Enabled)));
        assert!(is_enabled(&MockItem::new(RequiresApproval)));
        assert!(!is_enabled(&MockItem::new(NotRegistered)));
        assert!(!is_enabled(&MockItem::new(NotFound)));
    }

    #[test]
    fn enabling_registers_the_app() {
        let item = MockItem::new(NotRegistered);
        set_enabled(&item, true, true).unwrap();
        assert_eq!(item.status(), Enabled);
    }

    #[test]
    fn enabling_twice_does_not_register_again() {
        let item = MockItem::new(Enabled);
        set_enabled(&item, true, true).unwrap();
        assert!(item.calls.borrow().is_empty());
    }

    #[test]
    fn enabling_outside_a_bundle_is_refused_without_touching_the_os() {
        let item = MockItem::new(NotRegistered);
        assert_eq!(set_enabled(&item, true, false), Err(NOT_IN_BUNDLE_MESSAGE.to_string()));
        assert!(item.calls.borrow().is_empty());
    }

    #[test]
    fn enabling_asks_for_approval_when_the_os_requires_it() {
        let item = MockItem { after_register: RequiresApproval, ..MockItem::new(NotRegistered) };
        assert_eq!(set_enabled(&item, true, true), Err(APPROVAL_MESSAGE.to_string()));
    }

    #[test]
    fn enabling_reports_a_registration_error() {
        let item = MockItem { fail: Some("Operation not permitted"), ..MockItem::new(NotRegistered) };
        assert_eq!(set_enabled(&item, true, true), Err("Operation not permitted".to_string()));
    }

    #[test]
    fn disabling_unregisters_even_outside_a_bundle() {
        for status in [Enabled, RequiresApproval] {
            let item = MockItem::new(status);
            set_enabled(&item, false, false).unwrap();
            assert_eq!(item.status(), NotRegistered);
        }
    }

    #[test]
    fn disabling_when_not_registered_does_nothing() {
        for status in [NotRegistered, NotFound] {
            let item = MockItem::new(status);
            set_enabled(&item, false, true).unwrap();
            assert!(item.calls.borrow().is_empty());
        }
    }

    #[test]
    fn disabling_reports_an_unregistration_error() {
        let item = MockItem { fail: Some("boom"), ..MockItem::new(Enabled) };
        assert_eq!(set_enabled(&item, false, true), Err("boom".to_string()));
    }

    const LEGACY: &str = "/Users/me/Library/LaunchAgents/D-Bako.plist";
    const LEGACY_PLIST: &str = r#"<plist version="1.0"><dict><key>Label</key><string>D-Bako</string><key>ProgramArguments</key><array><string>/x/target/debug/d-bako</string></array></dict></plist>"#;

    fn fs_with(contents: &str) -> MockFs {
        let fs = MockFs::default();
        fs.files.borrow_mut().insert(PathBuf::from(LEGACY), contents.into());
        fs
    }

    #[test]
    fn removes_the_legacy_launch_agent() {
        let fs = fs_with(LEGACY_PLIST);
        assert_eq!(remove_legacy_launch_agent(&fs, Path::new(LEGACY)), Ok(true));
        assert!(fs.files.borrow().is_empty());
    }

    #[test]
    fn does_nothing_without_a_legacy_launch_agent() {
        assert_eq!(remove_legacy_launch_agent(&MockFs::default(), Path::new(LEGACY)), Ok(false));
    }

    #[test]
    fn keeps_a_launch_agent_that_is_not_ours() {
        let fs = fs_with(r#"<plist><dict><key>Label</key><string>com.example.other</string></dict></plist>"#);
        assert_eq!(remove_legacy_launch_agent(&fs, Path::new(LEGACY)), Ok(false));
        assert_eq!(fs.files.borrow().len(), 1);
    }

    #[test]
    fn reports_read_and_remove_errors() {
        assert!(remove_legacy_launch_agent(&MockFs::failing("read"), Path::new(LEGACY)).unwrap_err().contains("read denied"));
        let fs = fs_with(LEGACY_PLIST);
        fs.fail_on.set(Some("remove"));
        assert!(remove_legacy_launch_agent(&fs, Path::new(LEGACY)).unwrap_err().contains("remove denied"));
    }
}
