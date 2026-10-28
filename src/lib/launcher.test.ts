import { describe, expect, it } from "vitest";
import { clampPage, pickAppPath } from "./launcher";

describe("pickAppPath", () => {
  it("returns null for an empty list", () => {
    expect(pickAppPath([])).toBeNull();
  });

  it("prefers a .app bundle over other files", () => {
    expect(pickAppPath(["/tmp/a.txt", "/Applications/Foo.app"])).toBe(
      "/Applications/Foo.app",
    );
  });

  it("returns null when no .app bundle is present", () => {
    expect(pickAppPath(["/tmp/a.txt", "/tmp/b.txt"])).toBeNull();
  });
});

describe("clampPage", () => {
  it("keeps a page within range", () => {
    expect(clampPage(2, 5)).toBe(2);
  });

  it("clamps below zero to 0", () => {
    expect(clampPage(-1, 5)).toBe(0);
  });

  it("clamps beyond the last page to maxPages - 1", () => {
    expect(clampPage(9, 5)).toBe(4);
  });
});
