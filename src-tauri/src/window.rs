//! ウィンドウのサイズ定義。縦横比は基準サイズ (起動時のサイズ) で固定し、最小サイズも同じ比率にする。
//! OS には依存しない純粋な値のみを持つ。

/// 起動時のウィンドウサイズ (論理ピクセル)。`tauri.conf.json` の width / height と同じ値にする。
pub const BASE_WIDTH: f64 = 640.0;
pub const BASE_HEIGHT: f64 = 410.0;
/// これ以上は小さくできない幅。高さは縦横比から決まる。
pub const MIN_WIDTH: f64 = 480.0;

pub fn aspect_ratio() -> f64 { BASE_WIDTH / BASE_HEIGHT }

/// 最小サイズ (幅, 高さ)。基準サイズと同じ縦横比。
pub fn min_size() -> (f64, f64) { (MIN_WIDTH, MIN_WIDTH / aspect_ratio()) }

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;

    #[test]
    fn aspect_ratio_matches_the_base_size() {
        assert!((aspect_ratio() - BASE_WIDTH / BASE_HEIGHT).abs() < EPS);
    }

    #[test]
    fn min_size_keeps_the_aspect_ratio() {
        let (w, h) = min_size();
        assert!((w / h - aspect_ratio()).abs() < EPS);
    }

    #[test]
    fn min_size_is_smaller_than_the_base_size() {
        let (w, h) = min_size();
        assert!(w < BASE_WIDTH && h < BASE_HEIGHT);
    }

    #[test]
    fn min_size_is_large_enough_for_the_traffic_lights() {
        // 信号ボタン (約 28px) の領域が確保できる高さが最低限必要
        assert!(min_size().1 >= 200.0);
    }
}
