//! ウィンドウのサイズ定義。縦横比は基準サイズ (起動時のサイズ) で固定し、最小サイズも同じ比率にする。
//! OS には依存しない純粋な値のみを持つ。

/// 起動時のウィンドウサイズ (論理ピクセル)。`tauri.conf.json` の width / height と同じ値にする。
pub const BASE_WIDTH: f64 = 640.0;
pub const BASE_HEIGHT: f64 = 410.0;
/// これ以上は小さくできない幅。高さは縦横比から決まる。
pub const MIN_WIDTH: f64 = 320.0;

pub fn aspect_ratio() -> f64 { BASE_WIDTH / BASE_HEIGHT }

/// 最小サイズ (幅, 高さ)。基準サイズと同じ縦横比。
pub fn min_size() -> (f64, f64) { (MIN_WIDTH, MIN_WIDTH / aspect_ratio()) }

/// 前回保存したサイズから、起動時のサイズ (幅, 高さ) を決める。保存ファイルは外部入力なので検証する。
/// 保存が無い・不正な値なら基準サイズ。幅は最小幅〜画面の幅 (`max_width`) に収め、高さは縦横比から決める。
pub fn restore_size(saved_width: Option<f64>, max_width: Option<f64>) -> (f64, f64) {
    let valid = |w: &f64| w.is_finite() && *w > 0.0;
    let Some(mut width) = saved_width.filter(valid) else { return (BASE_WIDTH, BASE_HEIGHT) };
    if let Some(max) = max_width.filter(valid) { width = width.min(max); }
    width = width.max(MIN_WIDTH);
    (width, width / aspect_ratio())
}

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
    fn min_size_is_about_half_of_the_base_size() {
        let ratio = min_size().0 / BASE_WIDTH;
        assert!((0.45..=0.55).contains(&ratio), "ratio = {ratio}");
    }

    #[test]
    fn restore_size_uses_the_base_size_when_nothing_is_saved() {
        assert_eq!(restore_size(None, Some(1440.0)), (BASE_WIDTH, BASE_HEIGHT));
    }

    #[test]
    fn restore_size_keeps_the_saved_width_and_derives_the_height() {
        let (w, h) = restore_size(Some(800.0), Some(1440.0));
        assert_eq!(w, 800.0);
        assert!((w / h - aspect_ratio()).abs() < EPS);
    }

    #[test]
    fn restore_size_raises_a_too_small_width_to_the_minimum() {
        assert_eq!(restore_size(Some(100.0), Some(1440.0)), min_size());
    }

    #[test]
    fn restore_size_shrinks_a_width_larger_than_the_screen() {
        let (w, h) = restore_size(Some(3000.0), Some(1440.0));
        assert_eq!(w, 1440.0);
        assert!((w / h - aspect_ratio()).abs() < EPS);
    }

    #[test]
    fn restore_size_does_not_go_below_the_minimum_even_on_a_tiny_screen() {
        assert_eq!(restore_size(Some(800.0), Some(200.0)).0, MIN_WIDTH);
    }

    #[test]
    fn restore_size_keeps_the_saved_width_when_the_screen_is_unknown() {
        assert_eq!(restore_size(Some(3000.0), None).0, 3000.0);
    }

    #[test]
    fn restore_size_ignores_invalid_values() {
        for bad in [f64::NAN, f64::INFINITY, 0.0, -10.0] {
            assert_eq!(restore_size(Some(bad), Some(1440.0)), (BASE_WIDTH, BASE_HEIGHT), "{bad}");
        }
    }

    #[test]
    fn restore_size_ignores_an_invalid_screen_width() {
        assert_eq!(restore_size(Some(800.0), Some(f64::NAN)).0, 800.0);
    }

    #[test]
    fn min_size_is_large_enough_for_the_traffic_lights() {
        // 信号ボタン (約 28px) の領域が確保できる高さが最低限必要
        assert!(min_size().1 >= 200.0);
    }
}
