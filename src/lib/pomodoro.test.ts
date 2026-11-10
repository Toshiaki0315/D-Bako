import { describe, expect, it } from "vitest";
import { remaining } from "./countdown";
import { BREAK_MS, WORK_MS, createPomodoro, nextPhase, skipPhase } from "./pomodoro";

describe("pomodoro", () => {
  it("uses 25 minutes of work and 5 minutes of break", () => {
    expect(WORK_MS).toBe(25 * 60_000);
    expect(BREAK_MS).toBe(5 * 60_000);
  });

  it("starts with a stopped work phase and nothing completed", () => {
    const p = createPomodoro();
    expect(p.phase).toBe("work");
    expect(p.completed).toBe(0);
    expect(p.countdown.endsAt).toBeNull();
    expect(remaining(p.countdown, 0)).toBe(WORK_MS);
  });

  it("moves from work to a stopped break and counts the finished work", () => {
    const p = nextPhase(createPomodoro());
    expect(p.phase).toBe("break");
    expect(p.completed).toBe(1);
    expect(p.countdown.endsAt).toBeNull();
    expect(remaining(p.countdown, 0)).toBe(BREAK_MS);
  });

  it("moves from break back to work without counting", () => {
    const p = nextPhase(nextPhase(createPomodoro()));
    expect(p.phase).toBe("work");
    expect(p.completed).toBe(1);
    expect(remaining(p.countdown, 0)).toBe(WORK_MS);
  });

  it("skips to the next phase without counting skipped work", () => {
    const p = skipPhase(createPomodoro());
    expect(p.phase).toBe("break");
    expect(p.completed).toBe(0);
    expect(skipPhase(p).phase).toBe("work");
  });
});
