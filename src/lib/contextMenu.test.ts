import { describe, expect, it } from "vitest";
import { contextMenuPosition } from "./contextMenu";

describe("contextMenuPosition", () => {
  const menu = { width: 140, height: 40 };

  it("opens at the pointer when there is room", () => {
    expect(contextMenuPosition(100, 50, 640, 410, menu)).toEqual({ left: 100, top: 50 });
  });

  it("moves left so the menu stays inside the right edge", () => {
    expect(contextMenuPosition(600, 50, 640, 410, menu)).toEqual({ left: 500, top: 50 });
  });

  it("moves up so the menu stays inside the bottom edge", () => {
    expect(contextMenuPosition(100, 400, 640, 410, menu)).toEqual({ left: 100, top: 370 });
  });

  it("never goes past the top-left corner of a tiny window", () => {
    expect(contextMenuPosition(50, 20, 100, 30, menu)).toEqual({ left: 0, top: 0 });
  });
});
