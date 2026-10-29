export type Point = { x: number; y: number };

/** これより動かなければクリックとして扱う (px)。 */
export const DRAG_THRESHOLD = 4;

/** 押した位置からしきい値を超えて動いたらドラッグとみなす。 */
export function isDragGesture(start: Point, now: Point, threshold = DRAG_THRESHOLD): boolean {
  return Math.abs(now.x - start.x) > threshold || Math.abs(now.y - start.y) > threshold;
}

/** ウィンドウ (ビューポート) の外かどうか。 */
export function isOutsideWindow({ x, y }: Point, width: number, height: number): boolean {
  return x < 0 || y < 0 || x >= width || y >= height;
}

/** 要素 (またはその祖先) のスロット番号 (`data-slot`)。スロットの上でなければ null。 */
export function slotIndexOf(el: Element | null): number | null {
  const slot = el?.closest<HTMLElement>("[data-slot]");
  return slot ? Number(slot.dataset.slot) : null;
}

export type ItemDrop = { type: "remove" } | { type: "move"; to: number };

/** 登録済みアイテムをドロップした結果。ウィンドウ外なら解除、別のスロットなら移動 (埋まっていれば入れ替え)。 */
export function resolveItemDrop(from: number, target: { outside: boolean; slot: number | null }): ItemDrop | null {
  if (target.outside) return { type: "remove" };
  if (target.slot === null || target.slot === from) return null;
  return { type: "move", to: target.slot };
}

/** Tauri のドロップ位置 (物理ピクセル) を CSS ピクセルに直す。 */
export function toClientPoint(position: Point, scale: number): Point {
  const s = scale > 0 ? scale : 1;
  return { x: position.x / s, y: position.y / s };
}
