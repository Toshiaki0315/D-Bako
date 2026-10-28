//! URL ショートカットまわりの純粋ロジック (正規化・表示名・ショートカットファイルの解析)。
//! OS・ネットワークには依存しない。

use crate::core::AppItem;
use std::path::Path;
use url::Url;

/// 入力を http(s) の URL に正規化する。スキームが無ければ https とみなす。
/// 認証情報つきの URL や、`.` を含まないホスト名 (`localhost` を除く) は拒否する。
pub fn normalize_url(input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() { return Err("URL is empty".to_string()); }
    let with_scheme = if input.contains("://") { input.to_string() } else { format!("https://{input}") };
    let url = Url::parse(&with_scheme).map_err(|e| e.to_string())?;
    if !matches!(url.scheme(), "http" | "https") { return Err("Only http and https URLs are supported".to_string()); }
    if !url.username().is_empty() || url.password().is_some() { return Err("URL must not contain credentials".to_string()); }
    let host = url.host_str().ok_or_else(|| "URL has no host".to_string())?;
    if !host.contains('.') && host != "localhost" { return Err("Invalid host".to_string()); }
    Ok(url.to_string())
}

/// 正規化済みの URL から、スロットに登録するアイテムを作る。
pub fn url_item(normalized_url: &str, icon: Option<String>) -> AppItem {
    AppItem::url(normalized_url, display_name(normalized_url), icon)
}

/// `open` に渡してよい対象か。`.app` のパスと http(s) の URL だけを許可する
/// (任意のパスやオプション、`file:` などを起動させないため)。
pub fn is_launchable(target: &str) -> bool {
    (target.starts_with('/') && target.ends_with(".app")) || target.starts_with("http://") || target.starts_with("https://")
}

/// スロットに出す名前。ホスト名から先頭の `www.` を除いたもの。
pub fn display_name(url: &str) -> String {
    Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.strip_prefix("www.").unwrap_or(h).to_string()))
        .unwrap_or_else(|| url.to_string())
}

/// `.webloc` (XML / バイナリ plist) から URL を取り出す。
pub fn parse_webloc(data: &[u8]) -> Result<String, String> {
    let value = plist::Value::from_reader(std::io::Cursor::new(data)).map_err(|e| e.to_string())?;
    value
        .as_dictionary()
        .and_then(|d| d.get("URL"))
        .and_then(|v| v.as_string())
        .map(str::to_string)
        .ok_or_else(|| "URL not found in webloc".to_string())
}

/// `.url` (INI 形式の Internet Shortcut) から URL を取り出す。
pub fn parse_url_file(contents: &str) -> Result<String, String> {
    contents
        .lines()
        .filter_map(|l| l.split_once('='))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("URL"))
        .map(|(_, v)| v.trim().to_string())
        .ok_or_else(|| "URL not found in .url file".to_string())
}

/// ショートカットファイル (`.webloc` / `.url`) の中身から、正規化済みの URL を返す。
pub fn shortcut_url(path: &Path, data: &[u8]) -> Result<String, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let raw = match ext.as_str() {
        "webloc" => parse_webloc(data)?,
        "url" => parse_url_file(&String::from_utf8_lossy(data))?,
        _ => return Err("Unsupported shortcut file".to_string()),
    };
    normalize_url(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn normalize_keeps_a_full_https_url() {
        assert_eq!(normalize_url("https://example.com/a?b=1").unwrap(), "https://example.com/a?b=1");
    }

    #[test]
    fn normalize_adds_https_when_the_scheme_is_missing() {
        assert_eq!(normalize_url("example.com").unwrap(), "https://example.com/");
    }

    #[test]
    fn normalize_trims_whitespace_and_accepts_http() {
        assert_eq!(normalize_url("  http://example.com \n").unwrap(), "http://example.com/");
    }

    #[test]
    fn normalize_accepts_localhost() {
        assert_eq!(normalize_url("localhost:3000").unwrap(), "https://localhost:3000/");
    }

    #[test]
    fn normalize_rejects_empty_input() {
        assert_eq!(normalize_url("   ").unwrap_err(), "URL is empty");
    }

    #[test]
    fn normalize_rejects_other_schemes() {
        for input in ["file:///etc/passwd", "ftp://example.com", "javascript:alert(1)", "mailto:a@example.com"] {
            assert!(normalize_url(input).is_err(), "{input}");
        }
    }

    #[test]
    fn normalize_rejects_plain_words_and_text_with_spaces() {
        assert_eq!(normalize_url("hello").unwrap_err(), "Invalid host");
        assert!(normalize_url("hello world.com").is_err());
    }

    #[test]
    fn url_item_uses_the_display_name_and_keeps_the_icon() {
        let item = url_item("https://www.example.com/a", Some("data:image/png;base64,AA".into()));
        assert_eq!(item.kind, crate::core::ItemKind::Url);
        assert_eq!(item.path, "https://www.example.com/a");
        assert_eq!(item.name, "example.com");
        assert_eq!(item.icon.as_deref(), Some("data:image/png;base64,AA"));
        assert_eq!(url_item("https://example.com/", None).icon, None);
    }

    #[test]
    fn only_apps_and_http_urls_are_launchable() {
        for ok in ["/Applications/Safari.app", "https://example.com/", "http://localhost:3000/"] {
            assert!(is_launchable(ok), "{ok}");
        }
        for ng in ["/etc/passwd", "file:///etc/passwd", "ftp://example.com", "javascript:alert(1)", "-a Calculator", ""] {
            assert!(!is_launchable(ng), "{ng}");
        }
    }

    #[test]
    fn display_name_uses_the_host_without_www() {
        assert_eq!(display_name("https://www.example.com/path"), "example.com");
        assert_eq!(display_name("https://docs.example.com/"), "docs.example.com");
    }

    #[test]
    fn display_name_falls_back_to_the_input_when_unparsable() {
        assert_eq!(display_name("not a url"), "not a url");
    }

    #[test]
    fn webloc_xml_is_parsed() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict><key>URL</key><string>https://example.com/x</string></dict></plist>"#;
        assert_eq!(parse_webloc(xml).unwrap(), "https://example.com/x");
    }

    #[test]
    fn webloc_binary_plist_is_parsed() {
        let mut dict = plist::Dictionary::new();
        dict.insert("URL".into(), plist::Value::String("https://example.com/b".into()));
        let mut bytes = Vec::new();
        plist::to_writer_binary(&mut bytes, &plist::Value::Dictionary(dict)).unwrap();
        assert_eq!(parse_webloc(&bytes).unwrap(), "https://example.com/b");
    }

    #[test]
    fn webloc_without_url_key_is_an_error() {
        let xml = br#"<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict></dict></plist>"#;
        assert_eq!(parse_webloc(xml).unwrap_err(), "URL not found in webloc");
        assert!(parse_webloc(b"garbage").is_err());
    }

    #[test]
    fn internet_shortcut_url_is_parsed_case_insensitively() {
        let ini = "[InternetShortcut]\r\nurl=https://example.com/i\r\nIconIndex=0\r\n";
        assert_eq!(parse_url_file(ini).unwrap(), "https://example.com/i");
    }

    #[test]
    fn internet_shortcut_without_url_is_an_error() {
        assert_eq!(parse_url_file("[InternetShortcut]\n").unwrap_err(), "URL not found in .url file");
    }

    #[test]
    fn shortcut_file_dispatches_on_extension_and_normalizes() {
        assert_eq!(shortcut_url(Path::new("/a/Foo.url"), b"[InternetShortcut]\nURL=example.com\n").unwrap(), "https://example.com/");
        let xml = br#"<?xml version="1.0"?><plist version="1.0"><dict><key>URL</key><string>https://example.org</string></dict></plist>"#;
        assert_eq!(shortcut_url(Path::new("/a/Bar.WEBLOC"), xml).unwrap(), "https://example.org/");
    }

    #[test]
    fn shortcut_file_rejects_unknown_extension() {
        assert_eq!(shortcut_url(Path::new("/a/x.txt"), b"").unwrap_err(), "Unsupported shortcut file");
    }

    #[test]
    fn shortcut_file_with_unsafe_url_is_rejected() {
        assert!(shortcut_url(Path::new("/a/x.url"), b"URL=file:///etc/passwd\n").is_err());
    }
}
