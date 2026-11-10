import { describe, expect, it } from "vitest";
import { buildPages } from "./pages";

const ALL = { clock: true, pomodoro: true, timer: true };

describe("buildPages", () => {
  it("puts the launcher pages first and the visible widget pages after them", () => {
    expect(buildPages(2, ALL)).toEqual([
      { kind: "launcher", index: 0 },
      { kind: "launcher", index: 1 },
      { kind: "clock" },
      { kind: "pomodoro" },
      { kind: "timer" },
    ]);
  });

  it("leaves out hidden widget pages", () => {
    expect(buildPages(1, { clock: false, pomodoro: true, timer: false })).toEqual([
      { kind: "launcher", index: 0 },
      { kind: "pomodoro" },
    ]);
  });

  it("shows only the launcher when every widget is hidden or the settings are unknown", () => {
    const launcherOnly = [{ kind: "launcher", index: 0 }];
    expect(buildPages(1, { clock: false, pomodoro: false, timer: false })).toEqual(launcherOnly);
    expect(buildPages(1, undefined)).toEqual(launcherOnly);
  });
});
