import { describe, expect, it } from "vitest";
import { initialOf, letterColor, looksLikeUrl, pickShortcutFile } from "./link";

describe("looksLikeUrl", () => {
  it("accepts http(s) URLs and bare domains, ignoring surrounding whitespace", () => {
    for (const ok of ["https://example.com/a?b=1", "http://localhost:3000", "example.com", "  www.example.co.jp/path \n"]) {
      expect(looksLikeUrl(ok), ok).toBe(true);
    }
  });

  it("rejects ordinary text, empty input, and other schemes", () => {
    for (const ng of ["", "   ", "hello", "hello world.com", "two\nlines.com", "ftp://example.com", "file:///etc/passwd", "javascript:alert(1)"]) {
      expect(looksLikeUrl(ng), ng).toBe(false);
    }
  });
});

describe("pickShortcutFile", () => {
  it("returns the first .webloc or .url file, case-insensitively", () => {
    expect(pickShortcutFile(["/a/x.txt", "/a/Site.WEBLOC", "/a/b.url"])).toBe("/a/Site.WEBLOC");
    expect(pickShortcutFile(["/a/b.url"])).toBe("/a/b.url");
  });

  it("returns null when there is none", () => {
    expect(pickShortcutFile(["/a/x.txt", "/Applications/Foo.app"])).toBeNull();
    expect(pickShortcutFile([])).toBeNull();
  });
});

describe("initialOf", () => {
  it("returns the first character in upper case", () => {
    expect(initialOf("example.com")).toBe("E");
    expect(initialOf("日本語.jp")).toBe("日");
  });

  it("falls back to a placeholder for an empty name", () => {
    expect(initialOf("")).toBe("?");
  });
});

describe("letterColor", () => {
  it("is deterministic and different for different names", () => {
    expect(letterColor("example.com")).toBe(letterColor("example.com"));
    expect(letterColor("example.com")).not.toBe(letterColor("github.com"));
  });

  it("returns an hsl color", () => {
    expect(letterColor("anything")).toMatch(/^hsl\(\d{1,3}, \d+%, \d+%\)$/);
  });
});
