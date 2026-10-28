//! URL ショートカットの favicon 取得。HTTP は `HttpClient` トレイト越しに行い、テストではモックに差し替える。
//! 1) ページの `<link rel="icon">` → 2) `/favicon.ico` の順に試す。

use base64::{engine::general_purpose::STANDARD, Engine};
use url::Url;

const MAX_HTML_BYTES: usize = 256 * 1024;
const MAX_ICON_BYTES: usize = 512 * 1024;

pub trait HttpClient {
    /// `url` を GET し、本文を最大 `max_bytes` まで返す。
    fn get(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String>;
}

/// タグ文字列から属性値を取り出す (属性名は大文字小文字を区別せず、引用符の有無も許す)。
fn attr(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find(name) {
        let start = from + i;
        from = start + name.len();
        if start == 0 || !lower.as_bytes()[start - 1].is_ascii_whitespace() { continue; }
        let Some(rest) = tag[from..].trim_start().strip_prefix('=') else { continue };
        let rest = rest.trim_start();
        let value = match rest.chars().next()? {
            q @ ('"' | '\'') => { let r = &rest[1..]; &r[..r.find(q)?] }
            _ => &rest[..rest.find(|c: char| c.is_whitespace() || c == '>').unwrap_or(rest.len())],
        };
        return Some(value.to_string());
    }
    None
}

/// HTML から favicon の URL を探す。`rel="icon"` を優先し、無ければ `apple-touch-icon`。
pub fn find_icon_href(html: &str, base: &Url) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let (mut icon, mut apple) = (None, None);
    let mut pos = 0;
    while let Some(i) = lower[pos..].find("<link") {
        let start = pos + i;
        let end = lower[start..].find('>').map(|e| start + e).unwrap_or(lower.len());
        pos = end.max(start + 5);
        let tag = &html[start..end];
        let (Some(rel), Some(href)) = (attr(tag, "rel"), attr(tag, "href")) else { continue };
        let Ok(resolved) = base.join(&href) else { continue };
        if !matches!(resolved.scheme(), "http" | "https") { continue; }
        let rel = rel.to_ascii_lowercase();
        let mut tokens = rel.split_whitespace();
        if tokens.clone().any(|t| t == "icon") {
            icon.get_or_insert(resolved.to_string());
        } else if tokens.any(|t| t == "apple-touch-icon") {
            apple.get_or_insert(resolved.to_string());
        }
    }
    icon.or(apple)
}

/// 先頭のバイト列から画像の種類を判定する。画像でなければ None (SVG はスクリプトを含み得るので扱わない)。
pub fn sniff_mime(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', ..] => Some("image/png"),
        [0, 0, 1, 0, ..] => Some("image/x-icon"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..] => Some("image/webp"),
        _ => None,
    }
}

/// ページの favicon を取得し、`<img src>` に使える data URL で返す。取得できなければ None。
pub fn fetch_favicon<C: HttpClient>(client: &C, page_url: &str) -> Option<String> {
    let page = Url::parse(page_url).ok()?;
    let mut candidates = Vec::new();
    if let Ok(html) = client.get(page.as_str(), MAX_HTML_BYTES) {
        candidates.extend(find_icon_href(&String::from_utf8_lossy(&html), &page));
    }
    let fallback = page.join("/favicon.ico").ok()?.to_string();
    if !candidates.contains(&fallback) { candidates.push(fallback); }
    candidates.iter().find_map(|url| {
        let bytes = client.get(url, MAX_ICON_BYTES).ok()?;
        let mime = sniff_mime(&bytes)?;
        Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    const PNG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

    /// URL ごとに決めた応答を返すモック。要求された URL を記録する。
    #[derive(Default)]
    struct MockHttp {
        responses: HashMap<String, Vec<u8>>,
        requested: RefCell<Vec<String>>,
    }

    impl MockHttp {
        fn with(mut self, url: &str, body: &[u8]) -> Self { self.responses.insert(url.to_string(), body.to_vec()); self }
    }

    impl HttpClient for MockHttp {
        fn get(&self, url: &str, _max_bytes: usize) -> Result<Vec<u8>, String> {
            self.requested.borrow_mut().push(url.to_string());
            self.responses.get(url).cloned().ok_or_else(|| "404".to_string())
        }
    }

    fn base() -> url::Url { url::Url::parse("https://example.com/dir/page").unwrap() }

    #[test]
    fn finds_a_relative_icon_href_and_resolves_it() {
        let html = r#"<html><head><link rel="icon" href="/static/fav.png"></head></html>"#;
        assert_eq!(find_icon_href(html, &base()).unwrap(), "https://example.com/static/fav.png");
    }

    #[test]
    fn handles_shortcut_icon_single_quotes_and_uppercase_tags() {
        let html = "<LINK REL='shortcut icon' HREF='fav.ico'>";
        assert_eq!(find_icon_href(html, &base()).unwrap(), "https://example.com/dir/fav.ico");
    }

    #[test]
    fn resolves_protocol_relative_and_absolute_hrefs() {
        assert_eq!(find_icon_href(r#"<link rel="icon" href="//cdn.example.net/i.png">"#, &base()).unwrap(), "https://cdn.example.net/i.png");
        assert_eq!(find_icon_href(r#"<link rel="icon" href="http://other.example/i.png">"#, &base()).unwrap(), "http://other.example/i.png");
    }

    #[test]
    fn prefers_icon_over_apple_touch_icon_regardless_of_order() {
        let html = r#"<link rel="apple-touch-icon" href="/apple.png"><link rel="icon" href="/plain.png">"#;
        assert_eq!(find_icon_href(html, &base()).unwrap(), "https://example.com/plain.png");
    }

    #[test]
    fn falls_back_to_apple_touch_icon() {
        let html = r#"<link rel="apple-touch-icon" href="/apple.png">"#;
        assert_eq!(find_icon_href(html, &base()).unwrap(), "https://example.com/apple.png");
    }

    #[test]
    fn ignores_non_icon_links_and_unsafe_schemes() {
        assert_eq!(find_icon_href(r#"<link rel="stylesheet" href="/a.css">"#, &base()), None);
        assert_eq!(find_icon_href(r#"<link rel="icon" href="javascript:alert(1)">"#, &base()), None);
        assert_eq!(find_icon_href(r#"<link rel="icon">"#, &base()), None);
        assert_eq!(find_icon_href("no tags here", &base()), None);
    }

    #[test]
    fn sniffs_image_types_from_magic_bytes() {
        assert_eq!(sniff_mime(&PNG), Some("image/png"));
        assert_eq!(sniff_mime(&[0, 0, 1, 0, 1, 0]), Some("image/x-icon"));
        assert_eq!(sniff_mime(b"GIF89a"), Some("image/gif"));
        assert_eq!(sniff_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
        assert_eq!(sniff_mime(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff_mime(b"<svg></svg>"), None);
        assert_eq!(sniff_mime(b"<html>"), None);
        assert_eq!(sniff_mime(&[]), None);
    }

    #[test]
    fn fetches_the_icon_linked_from_the_page() {
        let http = MockHttp::default()
            .with("https://example.com/", br#"<link rel="icon" href="/fav.png">"#)
            .with("https://example.com/fav.png", &PNG);
        assert_eq!(fetch_favicon(&http, "https://example.com/").unwrap(), "data:image/png;base64,iVBORw0KGgo=");
    }

    #[test]
    fn falls_back_to_favicon_ico_when_the_page_cannot_be_read() {
        let http = MockHttp::default().with("https://example.com/favicon.ico", &[0, 0, 1, 0, 1, 0]);
        let icon = fetch_favicon(&http, "https://example.com/some/page").unwrap();
        assert!(icon.starts_with("data:image/x-icon;base64,"));
    }

    #[test]
    fn falls_back_when_the_linked_icon_is_missing_or_not_an_image() {
        let http = MockHttp::default()
            .with("https://example.com/", br#"<link rel="icon" href="/broken.png">"#)
            .with("https://example.com/broken.png", b"<html>not an image</html>")
            .with("https://example.com/favicon.ico", &PNG);
        let icon = fetch_favicon(&http, "https://example.com/").unwrap();
        assert!(icon.starts_with("data:image/png;base64,"));
        assert_eq!(http.requested.borrow().len(), 3);
    }

    #[test]
    fn does_not_request_favicon_ico_twice() {
        let http = MockHttp::default().with("https://example.com/", br#"<link rel="icon" href="/favicon.ico">"#);
        assert_eq!(fetch_favicon(&http, "https://example.com/"), None);
        assert_eq!(http.requested.borrow().iter().filter(|u| u.ends_with("/favicon.ico")).count(), 1);
    }

    #[test]
    fn returns_none_when_nothing_works_or_the_url_is_invalid() {
        assert_eq!(fetch_favicon(&MockHttp::default(), "https://example.com/"), None);
        assert_eq!(fetch_favicon(&MockHttp::default(), "not a url"), None);
    }
}
