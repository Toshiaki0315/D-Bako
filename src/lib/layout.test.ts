import { describe, expect, it } from "vitest";
import { layoutWidth } from "./layout";

describe("layoutWidth", () => {
  it("matches the base window width of the Rust side (window::base_size)", () => {
    expect(layoutWidth(4, 2)).toBe(642); // tauri.conf.json の幅
    expect(layoutWidth(8, 1)).toBe(1266);
    expect(layoutWidth(1, 6)).toBe(174);
  });

  it("does not depend on the number of rows", () => {
    expect(layoutWidth(3, 1)).toBe(layoutWidth(3, 6));
  });
});
