import { describe, expect, it } from "vitest";
import { addTime, createCountdown, formatDuration, isFinished, pause, remaining, reset, start } from "./countdown";

const MIN = 60_000;

describe("countdown", () => {
  it("starts stopped with the whole duration left", () => {
    const c = createCountdown(5 * MIN);
    expect(c.endsAt).toBeNull();
    expect(remaining(c, 1000)).toBe(5 * MIN);
    expect(isFinished(c, 1000)).toBe(false);
  });

  it("counts down from the moment it starts", () => {
    const c = start(createCountdown(5 * MIN), 1000);
    expect(remaining(c, 1000 + MIN)).toBe(4 * MIN);
    expect(isFinished(c, 1000 + 5 * MIN - 1)).toBe(false);
    expect(isFinished(c, 1000 + 5 * MIN)).toBe(true);
    expect(remaining(c, 1000 + 9 * MIN)).toBe(0);
  });

  it("never shows more than the time left at start, even for a moment just before it", () => {
    // 画面の時刻は少し古いことがあるので、開始より前の時刻で尋ねられても増えない
    const c = start(createCountdown(10 * MIN), 1000);
    expect(remaining(c, 800)).toBe(10 * MIN);
    expect(formatDuration(remaining(c, 800))).toBe("10:00");

    const resumed = start(pause(c, 1000 + MIN), 5000 + MIN);
    expect(remaining(resumed, 4800 + MIN)).toBe(9 * MIN);
  });

  it("keeps the time left while paused and resumes from it", () => {
    let c = start(createCountdown(5 * MIN), 0);
    c = pause(c, 2 * MIN);
    expect(remaining(c, 10 * MIN)).toBe(3 * MIN);
    c = start(c, 10 * MIN);
    expect(remaining(c, 11 * MIN)).toBe(2 * MIN);
  });

  it("ignores start while running and pause while stopped", () => {
    const running = start(createCountdown(MIN), 0);
    expect(start(running, 500)).toBe(running);
    const stopped = createCountdown(MIN);
    expect(pause(stopped, 500)).toBe(stopped);
  });

  it("does not start with nothing left", () => {
    const empty = createCountdown(0);
    expect(start(empty, 0)).toBe(empty);
  });

  it("goes back to the whole duration on reset", () => {
    const c = reset(pause(start(createCountdown(5 * MIN), 0), MIN));
    expect(c).toEqual(createCountdown(5 * MIN));
  });

  it("adds time to a stopped countdown and to its duration", () => {
    const c = addTime(createCountdown(0), 5 * MIN, 0);
    expect(remaining(c, 0)).toBe(5 * MIN);
    expect(reset(c)).toEqual(createCountdown(5 * MIN));
  });

  it("adds time to a running countdown without stopping it", () => {
    const c = addTime(start(createCountdown(MIN), 0), MIN, 30_000);
    expect(c.endsAt).not.toBeNull();
    expect(remaining(c, 30_000)).toBe(90_000);
  });
});

describe("formatDuration", () => {
  it("shows minutes and seconds, rounding a part second up", () => {
    expect(formatDuration(25 * MIN)).toBe("25:00");
    expect(formatDuration(61_000)).toBe("01:01");
    expect(formatDuration(500)).toBe("00:01");
    expect(formatDuration(0)).toBe("00:00");
  });

  it("adds hours once there is an hour or more", () => {
    expect(formatDuration(60 * MIN)).toBe("1:00:00");
    expect(formatDuration(90 * MIN + 5000)).toBe("1:30:05");
  });
});
