import { describe, expect, it } from "vitest";
import { tooltipPosition } from "./tooltip";

describe("tooltipPosition", () => {
  it("places the tooltip to the lower right of the cursor on the left half", () => {
    expect(tooltipPosition(100, 50, 640)).toEqual({ left: 112, top: 68, alignRight: false });
  });

  it("flips to the left of the cursor on the right half so it stays visible", () => {
    expect(tooltipPosition(500, 50, 640)).toEqual({ left: 488, top: 68, alignRight: true });
  });

  it("treats the exact middle as the left half", () => {
    expect(tooltipPosition(320, 0, 640).alignRight).toBe(false);
  });
});
