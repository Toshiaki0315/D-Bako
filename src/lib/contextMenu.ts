/** 右クリックメニューの左上座標。ポインタの位置に開き、ウィンドウからはみ出す分だけ内側へずらす。 */
export function contextMenuPosition(
  x: number,
  y: number,
  viewportWidth: number,
  viewportHeight: number,
  menu: { width: number; height: number },
) {
  return {
    left: Math.max(0, Math.min(x, viewportWidth - menu.width)),
    top: Math.max(0, Math.min(y, viewportHeight - menu.height)),
  };
}
