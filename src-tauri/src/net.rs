//! インフラ層: 実際の HTTP クライアント (ureq)。ネットワークに依存するため、カバレッジ計測の対象外。
//! ロジックは `favicon` 側にあり、そちらはモックでテストしている。

use crate::favicon::HttpClient;
use std::io::Read;
use std::time::Duration;

pub struct UreqClient(ureq::Agent);

impl UreqClient {
    pub fn new() -> Self {
        Self(ureq::AgentBuilder::new().timeout(Duration::from_secs(5)).user_agent("D-Bako").build())
    }
}

impl Default for UreqClient {
    fn default() -> Self { Self::new() }
}

impl HttpClient for UreqClient {
    fn get(&self, url: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
        let response = self.0.get(url).call().map_err(|e| e.to_string())?;
        let mut body = Vec::new();
        response.into_reader().take(max_bytes as u64).read_to_end(&mut body).map_err(|e| e.to_string())?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::favicon::fetch_favicon;

    /// 実ネットワークに接続するので通常は実行しない: `cargo test -- --ignored live`
    /// `DBAKO_DUMP_ICON=<path>` を指定すると、取得した data URL をそのファイルに書き出す。
    #[test]
    #[ignore = "needs network"]
    fn live_fetches_a_real_favicon() {
        let icon = fetch_favicon(&UreqClient::new(), "https://github.com/").expect("favicon should be fetched");
        assert!(icon.starts_with("data:image/"), "{}", &icon[..40.min(icon.len())]);
        if let Ok(path) = std::env::var("DBAKO_DUMP_ICON") { std::fs::write(path, icon).unwrap(); }
    }

    #[test]
    #[ignore = "needs network"]
    fn live_returns_none_for_an_unreachable_host() {
        assert_eq!(fetch_favicon(&UreqClient::new(), "https://nonexistent.invalid/"), None);
    }
}
