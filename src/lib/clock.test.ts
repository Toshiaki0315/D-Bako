import { describe, expect, it } from "vitest";
import { formatClock } from "./clock";

describe("formatClock", () => {
  it("shows the time with zero-padded hours, minutes and seconds", () => {
    expect(formatClock(new Date(2026, 10, 9, 7, 5, 3))).toMatchObject({ time: "07:05", seconds: "03" });
    expect(formatClock(new Date(2026, 10, 9, 23, 59, 59))).toMatchObject({ time: "23:59", seconds: "59" });
  });

  it("shows the date with the Japanese day of the week", () => {
    expect(formatClock(new Date(2026, 10, 9, 12, 0, 0)).date).toBe("2026年11月9日 (月)");
    expect(formatClock(new Date(2026, 10, 15, 12, 0, 0)).date).toBe("2026年11月15日 (日)");
  });
});
