/** ドロップされたパス群から登録対象の `.app` バンドルを選ぶ。無ければ null。 */
export function pickAppPath(paths: string[]): string | null {
  return paths.find((p) => p.endsWith(".app")) ?? null;
}

/** ページ番号を 0 〜 maxPages-1 に収める。端を越えたら反対側の端へ回り込む。 */
export function wrapPage(page: number, maxPages: number): number {
  return ((page % maxPages) + maxPages) % maxPages;
}
