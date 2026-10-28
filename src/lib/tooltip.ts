const OFFSET_X = 12;
const OFFSET_Y = 18;

/** マウス位置の近くに出す吹き出しの座標。右半分では左側に出して、画面外へはみ出さないようにする。 */
export function tooltipPosition(x: number, y: number, viewportWidth: number) {
  const alignRight = x > viewportWidth / 2;
  return { left: alignRight ? x - OFFSET_X : x + OFFSET_X, top: y + OFFSET_Y, alignRight };
}
