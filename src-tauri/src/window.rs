//! ウィンドウのサイズ定義。OS には依存しない純粋な値のみを持つ。
//!
//! 画面 (App.css) の寸法は「1rem = 16px × ウィンドウ幅 ÷ 基準の幅」で拡縮するが、タイトルバーとページ送りには
//! 小さくしたときの下限 (px) がある。そのため縦横比を一定にすると、小さいウィンドウほどグリッドの左右が空く。
//! そこで高さは、その幅でグリッドがちょうど収まる高さ (`height_for`) にする。下の定数は App.css と合わせる。

/// 4 × 2 のときの基準サイズ (論理ピクセル)。`tauri.conf.json` の width / height と同じ値にする。
pub const BASE_WIDTH: f64 = 642.0;
pub const BASE_HEIGHT: f64 = 406.0;

/// 基準の幅での 1rem。
const BASE_REM: f64 = 16.0;
/// 基準の幅の計算に使う 1 マスとすき間 (`lib/layout.ts` の `layoutWidth` と同じ)。
const SLOT: f64 = 150.0;
const GAP: f64 = 6.0;
const PAD_X: f64 = 24.0;
/// App.css の寸法 (rem)。左右の余白 (.container)、マスのすき間 (.grid)、タイトルバー (.drag-region)、
/// ページ送りの上の余白とボタン (.pager)、下の余白 (.container)。
const SIDE_REM: f64 = 0.75;
const GAP_REM: f64 = 0.375;
const TITLE_REM: f64 = 2.5;
const PAGER_PAD_REM: f64 = 0.625;
const PAGER_BUTTON_REM: f64 = 2.25;
const BOTTOM_REM: f64 = 0.875;
/// タイトルバー (信号ボタン) とページ送りのボタンが、小さくしてもそれより縮まない高さ (px)。
const TITLE_MIN: f64 = 28.0;
const PAGER_BUTTON_MIN: f64 = 16.0;
/// 最小サイズでの 1 マスの大きさ。
const MIN_SLOT: f64 = 30.0;
/// 信号ボタン (右端が約 70px) とページ送りが収まる最小の幅。1 列のグリッドの最小はこれで決まる。
const FLOOR_WIDTH: f64 = 80.0;

/// グリッドの横の数に合った基準の幅。UI の拡縮の基準になる。
pub fn base_width(cols: usize) -> f64 {
    let c = cols as f64;
    PAD_X + c * SLOT + (c - 1.0) * GAP
}

fn rem(width: f64, cols: usize) -> f64 { BASE_REM * width / base_width(cols) }

/// その幅のウィンドウでの 1 マスの大きさ (幅いっぱいに並べたとき)。
pub fn slot_size(width: f64, cols: usize) -> f64 {
    let (c, rem) = (cols as f64, rem(width, cols));
    (width - 2.0 * SIDE_REM * rem - (c - 1.0) * GAP_REM * rem) / c
}

/// その幅のウィンドウで、グリッドが上下左右ちょうどに収まる高さ。
pub fn height_for(width: f64, cols: usize, rows: usize) -> f64 {
    let (r, rem) = (rows as f64, rem(width, cols));
    let chrome = TITLE_MIN.max(TITLE_REM * rem) + PAGER_PAD_REM * rem + PAGER_BUTTON_MIN.max(PAGER_BUTTON_REM * rem) + BOTTOM_REM * rem;
    chrome + r * slot_size(width, cols) + (r - 1.0) * GAP_REM * rem
}

/// その幅での縦横比 (幅 ÷ 高さ)。大きさを変えるたびに設定し直す。
pub fn aspect_ratio_at(width: f64, cols: usize, rows: usize) -> f64 {
    width / height_for(width, cols, rows)
}

/// グリッドの大きさ (横・縦の数) に合った基準サイズ (幅, 高さ)。
pub fn base_size(cols: usize, rows: usize) -> (f64, f64) {
    let w = base_width(cols);
    (w, height_for(w, cols, rows))
}

/// 最小サイズ (幅, 高さ)。マスが 30px 角になる幅で、信号ボタンが収まる幅は確保する。
pub fn min_size(cols: usize, rows: usize) -> (f64, f64) {
    // マスの大きさは幅に比例するので、30px になる幅は割り算で求まる
    let width = (MIN_SLOT / slot_size(1.0, cols)).max(FLOOR_WIDTH);
    (width, height_for(width, cols, rows))
}

/// 高さが `height` を超えない最大の幅 (`height_for` は幅とともに増えるので二分法で求める)。
fn width_for_height(height: f64, cols: usize, rows: usize) -> f64 {
    let (mut lo, mut hi) = (0.0, 100_000.0);
    for _ in 0..100 {
        let mid = (lo + hi) / 2.0;
        if height_for(mid, cols, rows) <= height { lo = mid } else { hi = mid }
    }
    lo
}

/// 幅をもとに、画面 (`screen`: 幅, 高さ) に収まる大きさにする。最小サイズは下回らない。高さは幅から決める。
fn fit(width: f64, cols: usize, rows: usize, screen: Option<(f64, f64)>) -> (f64, f64) {
    let mut width = width;
    if let Some((sw, sh)) = screen.filter(|(w, h)| valid(w) && valid(h)) {
        width = width.min(sw).min(width_for_height(sh, cols, rows));
    }
    let width = width.max(min_size(cols, rows).0);
    (width, height_for(width, cols, rows))
}

fn valid(w: &f64) -> bool { w.is_finite() && *w > 0.0 }

/// 前回保存したサイズから、起動時のサイズ (幅, 高さ) を決める。保存ファイルは外部入力なので検証する。
/// 保存が無い・不正な値なら基準サイズ。どちらも画面に収め、最小サイズは下回らない。
pub fn restore_size(saved_width: Option<f64>, cols: usize, rows: usize, screen: Option<(f64, f64)>) -> (f64, f64) {
    let width = saved_width.filter(valid).unwrap_or_else(|| base_width(cols));
    fit(width, cols, rows, screen)
}

/// グリッドの大きさを変えたときの新しいウィンドウサイズ。マスの大きさを保ち、画面に収める。
pub fn resize_for_layout(current_width: f64, from: (usize, usize), to: (usize, usize), screen: Option<(f64, f64)>) -> (f64, f64) {
    let scale = current_width / base_width(from.0);
    let scale = if valid(&scale) { scale } else { 1.0 };
    fit(base_width(to.0) * scale, to.0, to.1, screen)
}

/// `size` (幅, 高さ) のウィンドウを、領域 (`origin`: 左上, `area`: 幅, 高さ) の中央に置く左上の位置。
/// 領域より大きければ左上に揃え、はみ出しは右下にする。単位はどれも物理ピクセル。
pub fn center_in(origin: (i32, i32), area: (u32, u32), size: (u32, u32)) -> (i32, i32) {
    let offset = |a: u32, s: u32| (a.saturating_sub(s) / 2) as i32;
    (origin.0 + offset(area.0, size.0), origin.1 + offset(area.1, size.1))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-6;
    const SCREEN: Option<(f64, f64)> = Some((1440.0, 900.0));
    const LAYOUTS: [(usize, usize); 8] = [(4, 2), (8, 1), (1, 6), (1, 1), (8, 6), (3, 3), (6, 2), (2, 4)];

    /// App.css と同じ計算で、その大きさのウィンドウに出る、グリッドの左右の空き (各辺) と上下の空き (合計)。
    fn css_slack(width: f64, height: f64, cols: usize, rows: usize) -> (f64, f64) {
        let (c, r) = (cols as f64, rows as f64);
        let rem = 16.0 * width / base_width(cols);
        let grid_w = width - 1.5 * rem;
        let grid_h = height - 28f64.max(2.5 * rem) - 0.625 * rem - 16f64.max(2.25 * rem) - 0.875 * rem;
        let gap = 0.375 * rem;
        let slot = ((grid_w - (c - 1.0) * gap) / c).min((grid_h - (r - 1.0) * gap) / r);
        ((grid_w - c * slot - (c - 1.0) * gap) / 2.0, grid_h - r * slot - (r - 1.0) * gap)
    }

    #[test]
    fn base_size_of_the_default_grid_matches_the_configured_window() {
        assert_eq!(base_size(4, 2), (BASE_WIDTH, BASE_HEIGHT));
        assert_eq!((BASE_WIDTH, BASE_HEIGHT), (642.0, 406.0));
    }

    #[test]
    fn base_width_grows_by_one_slot_and_gap_per_column() {
        assert_eq!(base_width(5), base_width(4) + 156.0);
        assert_eq!(base_width(8), 1266.0);
    }

    #[test]
    fn height_for_leaves_no_slack_at_any_width_or_layout() {
        for (c, r) in LAYOUTS {
            for scale in [0.1, 0.28, 0.5, 1.0, 1.7] {
                let w = base_width(c) * scale;
                let (side, vert) = css_slack(w, height_for(w, c, r), c, r);
                assert!(side.abs() < EPS && vert.abs() < EPS, "{c}x{r} at {w}: side {side}, vertical {vert}");
            }
        }
    }

    #[test]
    fn height_for_grows_with_the_width() {
        for (c, r) in LAYOUTS {
            assert!(height_for(500.0, c, r) > height_for(400.0, c, r), "{c}x{r}");
        }
    }

    #[test]
    fn slots_are_150px_at_the_base_size_of_every_layout() {
        for (c, r) in LAYOUTS {
            let slot = slot_size(base_width(c), c);
            assert!((slot - 150.0).abs() < EPS, "{c}x{r}: {slot}");
        }
    }

    #[test]
    fn min_size_shrinks_slots_to_30px_and_fits_the_grid() {
        for (c, r) in [(4, 2), (8, 1), (8, 6), (6, 2)] {
            let (w, h) = min_size(c, r);
            assert!((slot_size(w, c) - MIN_SLOT).abs() < EPS, "{c}x{r}");
            assert!((h - height_for(w, c, r)).abs() < EPS, "{c}x{r}");
        }
        assert_eq!(MIN_SLOT, 30.0);
    }

    #[test]
    fn min_size_keeps_room_for_the_traffic_lights() {
        for (c, r) in LAYOUTS {
            let (w, h) = min_size(c, r);
            assert!(w >= FLOOR_WIDTH - EPS, "{c}x{r}: {w}");
            assert!((h - height_for(w, c, r)).abs() < EPS, "{c}x{r}");
        }
        // 1 列は 30px のマスでは信号ボタンが収まらないので、幅の下限で決まる
        assert_eq!(min_size(1, 1).0, FLOOR_WIDTH);
    }

    #[test]
    fn min_size_of_a_single_column_fits_a_small_screen() {
        // 1 × 8 の最小でも、作業領域の高さ 800px の画面に収まる
        let (w, h) = min_size(1, 8);
        assert_eq!(w, FLOOR_WIDTH);
        assert!(h < 800.0, "{h}");
        assert_eq!(FLOOR_WIDTH, 80.0, "the traffic lights end at about 70px");
    }

    #[test]
    fn restore_size_uses_the_base_size_when_nothing_is_saved() {
        assert_eq!(restore_size(None, 4, 2, SCREEN), (BASE_WIDTH, BASE_HEIGHT));
    }

    #[test]
    fn restore_size_keeps_the_saved_width_and_fits_the_height_to_it() {
        let (w, h) = restore_size(Some(800.0), 4, 2, SCREEN);
        assert_eq!(w, 800.0);
        assert!((h - height_for(800.0, 4, 2)).abs() < EPS);
    }

    #[test]
    fn restore_size_raises_a_too_small_width_to_the_minimum() {
        assert_eq!(restore_size(Some(100.0), 4, 2, SCREEN), min_size(4, 2));
    }

    #[test]
    fn restore_size_shrinks_a_width_larger_than_the_screen() {
        let (w, h) = restore_size(Some(3000.0), 4, 2, Some((1440.0, 2000.0)));
        assert_eq!(w, 1440.0);
        assert!((h - height_for(1440.0, 4, 2)).abs() < EPS);
    }

    #[test]
    fn restore_size_shrinks_a_window_taller_than_the_screen() {
        let (w, h) = restore_size(None, 8, 6, SCREEN);
        assert!((h - 900.0).abs() < 0.01, "{h}");
        assert!((h - height_for(w, 8, 6)).abs() < EPS);
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
        let (w, h) = resize_for_layout(BASE_WIDTH * 1.2, (4, 2), (5, 3), Some((4000.0, 4000.0)));
        assert!((slot_size(w, 5) - slot_size(BASE_WIDTH * 1.2, 4)).abs() < EPS);
        assert!((h - height_for(w, 5, 3)).abs() < EPS);
    }

    #[test]
    fn resize_for_layout_fits_the_screen() {
        let (w, h) = resize_for_layout(BASE_WIDTH, (4, 2), (8, 6), SCREEN);
        assert!(w <= 1440.0 + EPS && h <= 900.0 + 0.01);
        assert!((h - height_for(w, 8, 6)).abs() < EPS);
    }

    #[test]
    fn resize_for_layout_falls_back_to_the_base_size_for_an_invalid_width() {
        assert_eq!(resize_for_layout(f64::NAN, (4, 2), (3, 3), SCREEN), base_size(3, 3));
    }

    #[test]
    fn center_in_places_the_window_in_the_middle_of_the_area() {
        assert_eq!(center_in((0, 0), (1000, 800), (400, 200)), (300, 300));
    }

    #[test]
    fn center_in_works_on_a_display_with_negative_coordinates() {
        // メイン画面の左上にある外部ディスプレイ
        assert_eq!(center_in((-657, -1692), (3008, 1692), (720, 376)), (-657 + 1144, -1692 + 658));
    }

    #[test]
    fn center_in_aligns_a_window_larger_than_the_area_to_its_top_left() {
        assert_eq!(center_in((10, 20), (300, 100), (400, 200)), (10, 20));
    }
}
