/**
 * グリッドの横の数に合った、基準のウィンドウ幅 (px)。Rust 側の `window::base_size` の幅と同じ計算にする
 * (左右の余白 24px + マス 150px × 横の数 + すき間 6px)。UI 全体は「今の幅 ÷ この幅」の倍率で拡縮する。
 */
export function layoutWidth(cols: number, _rows: number): number {
  return 24 + cols * 150 + (cols - 1) * 6;
}
