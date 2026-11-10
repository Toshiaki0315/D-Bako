import { createCountdown } from "./countdown";
import type { Countdown } from "./countdown";

/** 作業 25 分・休憩 5 分 (固定)。 */
export const WORK_MS = 25 * 60_000;
export const BREAK_MS = 5 * 60_000;

export type Phase = "work" | "break";
/** 今の段階 (作業 / 休憩)、その残り時間、終えた作業の回数。 */
export type Pomodoro = { phase: Phase; countdown: Countdown; completed: number };

export function createPomodoro(): Pomodoro {
  return { phase: "work", countdown: createCountdown(WORK_MS), completed: 0 };
}

/** 次の段階へ移る (止まった状態で)。作業を終えたときだけ回数を数える。 */
export function nextPhase(p: Pomodoro): Pomodoro {
  return p.phase === "work"
    ? { phase: "break", countdown: createCountdown(BREAK_MS), completed: p.completed + 1 }
    : { phase: "work", countdown: createCountdown(WORK_MS), completed: p.completed };
}

/** 途中で次の段階へ移る。終えていない作業は回数に数えない。 */
export function skipPhase(p: Pomodoro): Pomodoro {
  return { ...nextPhase(p), completed: p.completed };
}
