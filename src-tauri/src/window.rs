//! ウィンドウのサイズ定義。縦横比はグリッドの横・縦の数から決めた基準サイズで固定し、最小サイズも同じ比率にする。
//! OS には依存しない純粋な値のみを持つ。

/// 4 × 2 のときの基準サイズ (論理ピクセル)。`tauri.conf.json` の width / height と同じ値にする。
pub const BASE_WIDTH: f64 = 640.0;
pub const BASE_HEIGHT: f64 = 410.0;

/// 基準サイズでの 1 マスの大きさと、マスどうしのすき間。
const SLOT: f64 = 150.0;
const GAP: f64 = 6.0;
/// グリッド以外に要る幅 (左右の余白) と高さ (タイトルバー・ページ送り・下の余白)。
const PAD_X: f64 = 22.0;
const CHROME_Y: f64 = 104.0;
/// 基準サイズの半分までは小さくできる。
const MIN_SCALE: f64 = 0.5;
/// 信号ボタンとページ送りが収まる最小の幅・高さ。
const FLOOR_WIDTH: f64 = 200.0;
const FLOOR_HEIGHT: f64 = 205.0;

/// グリッドの大きさ (横・縦の数) に合った基準サイズ (幅, 高さ)。
pub fn base_size(cols: usize, rows: usize) -> (f64, f64) {
    let (c, r) = (cols as f64, rows as f64);
    (PAD_X + c * SLOT + (c - 1.0) * GAP, CHROME_Y + r * SLOT + (r - 1.0) * GAP)
}

pub fn aspect_ratio(cols: usize, rows: usize) -> f64 {
    let (w, h) = base_size(cols, rows);
    w / h
}

/// 最小サイズ (幅, 高さ)。基準サイズと同じ縦横比で、信号ボタンとページ送りが収まる大きさは確保する。
pub fn min_size(cols: usize, rows: usize) -> (f64, f64) {
    let (w, h) = base_size(cols, rows);
    let scale = MIN_SCALE.max(FLOOR_WIDTH / w).max(FLOOR_HEIGHT / h);
    (w * scale, h * scale)
}

/// 幅をもとに、縦横比を保って画面 (`screen`: 幅, 高さ) に収まる大きさにする。最小サイズは下回らない。
fn fit(width: f64, cols: usize, rows: usize, screen: Option<(f64, f64)>) -> (f64, f64) {
    let ratio = aspect_ratio(cols, rows);
    let mut width = width;
    if let Some((sw, sh)) = screen.filter(|(w, h)| valid(w) && valid(h)) {
        width = width.min(sw).min(sh * ratio);
    }
    let width = width.max(min_size(cols, rows).0);
    (width, width / ratio)
}

fn valid(w: &f64) -> bool { w.is_finite() && *w > 0.0 }

/// 前回保存したサイズから、起動時のサイズ (幅, 高さ) を決める。保存ファイルは外部入力なので検証する。
/// 保存が無い・不正な値なら基準サイズ。どちらも画面に収め、最小サイズは下回らない。高さは縦横比から決める。
pub fn restore_size(saved_width: Option<f64>, cols: usize, rows: usize, screen: Option<(f64, f64)>) -> (f64, f64) {
    let width = saved_width.filter(valid).unwrap_or_else(|| base_size(cols, rows).0);
    fit(width, cols, rows, screen)
}

/// グリッドの大きさを変えたときの新しいウィンドウサイズ。マスの大きさを保ち、画面に収める。
pub fn resize_for_layout(current_width: f64, from: (usize, usize), to: (usize, usize), screen: Option<(f64, f64)>) -> (f64, f64) {
    let scale = current_width / base_size(from.0, from.1).0;
    let scale = if valid(&scale) { scale } else { 1.0 };
    fit(base_size(to.0, to.1).0 * scale, to.0, to.1, screen)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-9;
    const SCREEN: Option<(f64, f64)> = Some((1440.0, 900.0));

    fn ratio_of((w, h): (f64, f64)) -> f64 { w / h }

    #[test]
    fn base_size_of_the_default_grid_matches_the_configured_window() {
        assert_eq!(base_size(4, 2), (BASE_WIDTH, BASE_HEIGHT));
    }

    #[test]
    fn base_size_grows_by_one_slot_and_gap_per_column_and_row() {
        let (w, h) = base_size(4, 2);
        assert_eq!(base_size(5, 3), (w + SLOT + GAP, h + SLOT + GAP));
    }

    #[test]
    fn min_size_of_the_default_grid_is_half_of_the_base_size() {
        assert_eq!(min_size(4, 2), (BASE_WIDTH / 2.0, BASE_HEIGHT / 2.0));
    }

    #[test]
    fn min_size_keeps_the_aspect_ratio() {
        for (c, r) in [(4, 2), (1, 1), (8, 6), (1, 6), (8, 1)] {
            assert!((ratio_of(min_size(c, r)) - aspect_ratio(c, r)).abs() < EPS, "{c}x{r}");
        }
    }

    #[test]
    fn min_size_leaves_room_for_the_traffic_lights_and_the_pager() {
        for (c, r) in [(1, 1), (1, 6), (8, 1), (8, 6)] {
            let (w, h) = min_size(c, r);
            assert!(w >= FLOOR_WIDTH - EPS && h >= FLOOR_HEIGHT - EPS, "{c}x{r}: {w}x{h}");
        }
    }

    #[test]
    fn restore_size_uses_the_base_size_when_nothing_is_saved() {
        assert_eq!(restore_size(None, 4, 2, SCREEN), (BASE_WIDTH, BASE_HEIGHT));
    }

    #[test]
    fn restore_size_keeps_the_saved_width_and_derives_the_height() {
        let (w, h) = restore_size(Some(800.0), 4, 2, SCREEN);
        assert_eq!(w, 800.0);
        assert!((w / h - aspect_ratio(4, 2)).abs() < EPS);
    }

    #[test]
    fn restore_size_raises_a_too_small_width_to_the_minimum() {
        assert_eq!(restore_size(Some(100.0), 4, 2, SCREEN), min_size(4, 2));
    }

    #[test]
    fn restore_size_shrinks_a_width_larger_than_the_screen() {
        let (w, h) = restore_size(Some(3000.0), 4, 2, Some((1440.0, 2000.0)));
        assert_eq!(w, 1440.0);
        assert!((w / h - aspect_ratio(4, 2)).abs() < EPS);
    }

    #[test]
    fn restore_size_shrinks_a_window_taller_than_the_screen() {
        let (_, h) = restore_size(None, 8, 6, SCREEN);
        assert!((h - 900.0).abs() < EPS);
    }

    #[test]
    fn restore_size_does_not_go_below_the_minimum_even_on_a_tiny_screen() {
        assert_eq!(restore_size(Some(800.0), 4, 2, Some((200.0, 100.0))), min_size(4, 2));
    }

    #[test]
    fn restore_size_keeps_the_saved_width_when_the_screen_is_unknown() {
        assert_eq!(restore_size(Some(3000.0), 4, 2, None).0, 3000.0);
    }

    #[test]
    fn restore_size_ignores_invalid_values() {
        for bad in [f64::NAN, f64::INFINITY, 0.0, -10.0] {
            assert_eq!(restore_size(Some(bad), 4, 2, SCREEN), (BASE_WIDTH, BASE_HEIGHT), "{bad}");
        }
    }

    #[test]
    fn restore_size_ignores_an_invalid_screen_size() {
        assert_eq!(restore_size(Some(800.0), 4, 2, Some((f64::NAN, 900.0))).0, 800.0);
        assert_eq!(restore_size(Some(800.0), 4, 2, Some((1440.0, 0.0))).0, 800.0);
    }

    #[test]
    fn resize_for_layout_keeps_the_slot_size() {
        // 基準サイズの 1.2 倍で使っていれば、新しいグリッドでも 1.2 倍にする
        let (w, h) = resize_for_layout(BASE_WIDTH * 1.2, (4, 2), (5, 3), Some((4000.0, 4000.0)));
        let (bw, bh) = base_size(5, 3);
        assert!((w - bw * 1.2).abs() < EPS && (h - bh * 1.2).abs() < EPS);
    }

    #[test]
    fn resize_for_layout_fits_the_screen() {
        let (w, h) = resize_for_layout(BASE_WIDTH, (4, 2), (8, 6), SCREEN);
        assert!(w <= 1440.0 + EPS && h <= 900.0 + EPS);
        assert!((w / h - aspect_ratio(8, 6)).abs() < EPS);
    }

    #[test]
    fn resize_for_layout_falls_back_to_the_base_size_for_an_invalid_width() {
        assert_eq!(resize_for_layout(f64::NAN, (4, 2), (3, 3), SCREEN), base_size(3, 3));
    }
}
