import { describe, expect, it } from "vitest";
import { pickAppPath, wrapPage } from "./launcher";

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

describe("wrapPage", () => {
  it("keeps a page within range", () => {
    expect(wrapPage(2, 5)).toBe(2);
  });

  it("goes from the first page back to the last", () => {
    expect(wrapPage(-1, 10)).toBe(9);
  });

  it("goes from the last page on to the first", () => {
    expect(wrapPage(10, 10)).toBe(0);
  });

  it("stays on the only page", () => {
    expect(wrapPage(1, 1)).toBe(0);
    expect(wrapPage(-1, 1)).toBe(0);
  });
});
