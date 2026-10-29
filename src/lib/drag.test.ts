import { describe, expect, it } from "vitest";
import { isDragGesture, isOutsideWindow, resolveItemDrop, slotIndexOf, toClientPoint } from "./drag";

describe("isDragGesture", () => {
  it("treats a small wobble as a click", () => {
    expect(isDragGesture({ x: 10, y: 10 }, { x: 13, y: 12 })).toBe(false);
  });

  it("starts a drag once the pointer moves past the threshold in any direction", () => {
    expect(isDragGesture({ x: 10, y: 10 }, { x: 15, y: 10 })).toBe(true);
    expect(isDragGesture({ x: 10, y: 10 }, { x: 10, y: 5 })).toBe(true);
  });
});

describe("isOutsideWindow", () => {
  it("is false inside the viewport, including its top-left edge", () => {
    expect(isOutsideWindow({ x: 0, y: 0 }, 640, 410)).toBe(false);
    expect(isOutsideWindow({ x: 639, y: 409 }, 640, 410)).toBe(false);
  });

  it("is true past any edge", () => {
    expect(isOutsideWindow({ x: -1, y: 10 }, 640, 410)).toBe(true);
    expect(isOutsideWindow({ x: 10, y: -1 }, 640, 410)).toBe(true);
    expect(isOutsideWindow({ x: 640, y: 10 }, 640, 410)).toBe(true);
    expect(isOutsideWindow({ x: 10, y: 410 }, 640, 410)).toBe(true);
  });
});

describe("slotIndexOf", () => {
  it("reads the slot index from the element or its ancestor", () => {
    const slot = document.createElement("div");
    slot.dataset.slot = "5";
    const icon = document.createElement("img");
    slot.appendChild(icon);
    expect(slotIndexOf(slot)).toBe(5);
    expect(slotIndexOf(icon)).toBe(5);
  });

  it("returns null outside any slot", () => {
    expect(slotIndexOf(document.createElement("div"))).toBeNull();
    expect(slotIndexOf(null)).toBeNull();
  });
});

describe("resolveItemDrop", () => {
  it("removes the item when dropped outside the window", () => {
    expect(resolveItemDrop(2, { outside: true, slot: null })).toEqual({ type: "remove" });
  });

  it("moves the item onto another slot (the backend swaps if it is occupied)", () => {
    expect(resolveItemDrop(2, { outside: false, slot: 6 })).toEqual({ type: "move", to: 6 });
  });

  it("does nothing when dropped on its own slot or on something that is not a slot", () => {
    expect(resolveItemDrop(2, { outside: false, slot: 2 })).toBeNull();
    expect(resolveItemDrop(2, { outside: false, slot: null })).toBeNull();
  });
});

describe("toClientPoint", () => {
  it("converts a physical drop position to CSS pixels", () => {
    expect(toClientPoint({ x: 200, y: 100 }, 2)).toEqual({ x: 100, y: 50 });
  });

  it("falls back to a scale of 1 for an invalid scale", () => {
    expect(toClientPoint({ x: 200, y: 100 }, 0)).toEqual({ x: 200, y: 100 });
  });
});
