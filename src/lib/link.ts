/** 貼り付けられた文字列が URL (http(s) または `example.com` のようなドメイン) に見えるか。普通の文章は弾く。 */
export function looksLikeUrl(text: string): boolean {
  const t = text.trim();
  if (!t || /\s/.test(t)) return false;
  if (/^https?:\/\/\S+$/i.test(t)) return true;
  if (t.includes("://")) return false;
  return /^localhost(:\d+)?([/?#]|$)/i.test(t) || /^[^\s/:?#@]+\.[a-z]{2,}(:\d+)?([/?#]|$)/i.test(t);
}

/** ドロップされたパス群から、URL ショートカットのファイル (`.webloc` / `.url`) を選ぶ。 */
export function pickShortcutFile(paths: string[]): string | null {
  return paths.find((p) => /\.(webloc|url)$/i.test(p)) ?? null;
}

/** favicon が無いときのアイコンに使う頭文字。 */
export function initialOf(name: string): string {
  return [...name][0]?.toUpperCase() ?? "?";
}

/** 名前から決まる (同じ名前なら同じ) アイコンの背景色。 */
export function letterColor(name: string): string {
  let hash = 0;
  for (const ch of name) hash = (hash * 31 + ch.charCodeAt(0)) >>> 0;
  return `hsl(${hash % 360}, 55%, 42%)`;
}
