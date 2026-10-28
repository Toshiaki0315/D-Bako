/** ドロップされたパス群から登録対象の `.app` バンドルを選ぶ。無ければ null。 */
export function pickAppPath(paths: string[]): string | null {
  return paths.find((p) => p.endsWith(".app")) ?? null;
}

/** ページ番号を 0 〜 maxPages-1 の範囲に収める。 */
export function clampPage(page: number, maxPages: number): number {
  return Math.min(Math.max(0, page), maxPages - 1);
}
