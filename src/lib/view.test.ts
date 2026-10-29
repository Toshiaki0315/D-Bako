import { describe, expect, it } from "vitest";
import { viewFromSearch } from "./view";

describe("viewFromSearch", () => {
  it("shows the launcher by default", () => {
    expect(viewFromSearch("")).toBe("launcher");
    expect(viewFromSearch("?foo=bar")).toBe("launcher");
  });

  it("shows the settings view in the settings window", () => {
    expect(viewFromSearch("?view=settings")).toBe("settings");
  });

  it("falls back to the launcher for an unknown view", () => {
    expect(viewFromSearch("?view=unknown")).toBe("launcher");
  });
});
