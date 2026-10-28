/** ドロップされたパス群から登録対象を選ぶ。`.app` を優先し、無ければ先頭を返す。 */
export function pickAppPath(paths: string[]): string | null {
  if (paths.length === 0) return null;
  return paths.find((p) => p.endsWith(".app")) ?? paths[0];
}

/** ページ番号を 0 〜 maxPages-1 の範囲に収める。 */
export function clampPage(page: number, maxPages: number): number {
  return Math.min(Math.max(0, page), maxPages - 1);
}
