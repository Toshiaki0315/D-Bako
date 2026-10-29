export type View = "launcher" | "settings";

/** ウィンドウごとに表示する画面。設定ウィンドウは `index.html?view=settings` で開かれる。 */
export function viewFromSearch(search: string): View {
  return new URLSearchParams(search).get("view") === "settings" ? "settings" : "launcher";
}
