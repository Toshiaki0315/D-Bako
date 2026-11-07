/** 起動時の状態の読み込みを、失敗したら何度か待ってやり直す間隔 (ミリ秒)。 */
export const LOAD_RETRY_DELAYS = [300, 600, 1200, 2400];

const wait = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

/** `fn` が失敗したら、`delays` の間隔を空けてやり直す。使い切っても失敗したら最後のエラーを投げる。 */
export async function retry<T>(fn: () => Promise<T>, delays: number[], sleep: (ms: number) => Promise<void> = wait): Promise<T> {
  for (let attempt = 0; ; attempt++) {
    try {
      return await fn();
    } catch (e) {
      if (attempt >= delays.length) throw e;
      await sleep(delays[attempt]);
    }
  }
}
