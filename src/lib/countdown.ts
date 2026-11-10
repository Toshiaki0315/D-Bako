/**
 * 残り時間を数えるタイマー。時刻 (`now`, ミリ秒) を引数で受け取る純粋な関数だけで扱う。
 * 動いている間は終わる時刻 (`endsAt`) を持つので、画面の更新が遅れたりページを移ったりしても、時刻のずれは積み重ならない。
 * `remainingMs` は、止まっている間は残り時間、動いている間は開始したときの残り時間。
 */
export type Countdown = { durationMs: number; remainingMs: number; endsAt: number | null };

export function createCountdown(durationMs: number): Countdown {
  return { durationMs, remainingMs: durationMs, endsAt: null };
}

/** 残り時間。画面の時刻は少し古いことがあるので、開始したときの残り時間より長くはしない (10:00 が 10:01 に見えないように)。 */
export function remaining(c: Countdown, now: number): number {
  return c.endsAt === null ? c.remainingMs : Math.min(c.remainingMs, Math.max(0, c.endsAt - now));
}

export function isFinished(c: Countdown, now: number): boolean {
  return c.endsAt !== null && now >= c.endsAt;
}

/** 止まっていれば、残り時間から数え始める。動いている・残りが無いときはそのまま。 */
export function start(c: Countdown, now: number): Countdown {
  if (c.endsAt !== null || c.remainingMs <= 0) return c;
  return { ...c, endsAt: now + c.remainingMs };
}

/** 動いていれば、その時点の残り時間で止める。 */
export function pause(c: Countdown, now: number): Countdown {
  if (c.endsAt === null) return c;
  return { ...c, remainingMs: remaining(c, now), endsAt: null };
}

/** 止めて、設定した時間に戻す。 */
export function reset(c: Countdown): Countdown {
  return createCountdown(c.durationMs);
}

/** 時間を足す。止まっていれば設定した時間にも足し、動いていれば止めずに終わる時刻を延ばす。 */
export function addTime(c: Countdown, ms: number, now: number): Countdown {
  if (c.endsAt !== null) {
    return { durationMs: c.durationMs + ms, remainingMs: remaining(c, now) + ms, endsAt: Math.max(c.endsAt, now) + ms };
  }
  return { durationMs: c.durationMs + ms, remainingMs: c.remainingMs + ms, endsAt: null };
}

const pad = (n: number) => String(n).padStart(2, "0");

/** 残り時間を「分:秒」(1 時間以上は「時:分:秒」) で表す。1 秒未満は切り上げる (0 になるまで 00:00 にしない)。 */
export function formatDuration(ms: number): string {
  const total = Math.ceil(ms / 1000);
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = total % 60;
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}
