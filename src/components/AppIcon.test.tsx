import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { AppIcon, clearIconCache } from "./AppIcon";

const URL_A = "data:image/png;base64,AAAA";
const URL_B = "data:image/png;base64,BBBB";

beforeEach(() => {
  invokeMock.mockReset();
  clearIconCache();
});

describe("AppIcon", () => {
  it("shows the fallback first, then the extracted icon image", async () => {
    invokeMock.mockResolvedValue(URL_A);
    render(<AppIcon path="/Applications/Foo.app" name="Foo" />);
    expect(screen.getByText("📱")).toBeInTheDocument();

    const img = await screen.findByRole("img", { name: "Foo" });
    expect(img).toHaveAttribute("src", URL_A);
    expect(img).toHaveClass("app-icon");
    expect(screen.queryByText("📱")).not.toBeInTheDocument();
    expect(invokeMock).toHaveBeenCalledWith("get_app_icon", {
      path: "/Applications/Foo.app",
    });
  });

  it("uses the stored favicon for a URL shortcut without asking the backend", () => {
    render(<AppIcon path="https://example.com/" name="example.com" kind="url" icon={URL_A} />);
    expect(screen.getByRole("img", { name: "example.com" })).toHaveAttribute("src", URL_A);
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("shows a colored initial for a URL shortcut that has no favicon", () => {
    render(<AppIcon path="https://example.com/" name="example.com" kind="url" />);
    const tile = screen.getByText("E");
    expect(tile).toHaveClass("app-icon", "app-icon-letter");
    expect(tile.style.background).toMatch(/^(hsl|rgb)/); // jsdom は hsl を rgb に変換する
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("falls back to the initial when a stored favicon fails to load", () => {
    render(<AppIcon path="https://example.com/" name="example.com" kind="url" icon="data:image/png;base64,broken" />);
    fireEvent.error(screen.getByRole("img", { name: "example.com" }));
    expect(screen.getByText("E")).toBeInTheDocument();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("keeps the fallback when extraction fails", async () => {
    invokeMock.mockRejectedValue("Icon file not found");
    render(<AppIcon path="/Applications/Foo.app" name="Foo" />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    expect(screen.getByText("📱")).toBeInTheDocument();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("keeps the fallback when the result is not a string", async () => {
    invokeMock.mockResolvedValue(undefined);
    render(<AppIcon path="/Applications/Foo.app" name="Foo" />);
    await waitFor(() => expect(invokeMock).toHaveBeenCalled());
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });

  it("reuses a cached icon instead of invoking again", async () => {
    invokeMock.mockResolvedValue(URL_A);
    const first = render(<AppIcon path="/Foo.app" name="Foo" />);
    await first.findByRole("img");
    first.unmount();

    render(<AppIcon path="/Foo.app" name="Foo" />);
    expect(screen.getByRole("img", { name: "Foo" })).toHaveAttribute("src", URL_A);
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("ignores a stale result after the path changes", async () => {
    let resolveA!: (v: string) => void;
    invokeMock.mockImplementation((_cmd: string, { path }: { path: string }) =>
      path === "/A.app"
        ? new Promise<string>((r) => (resolveA = r))
        : Promise.resolve(URL_B),
    );
    const { rerender } = render(<AppIcon path="/A.app" name="A" />);
    rerender(<AppIcon path="/B.app" name="B" />);
    expect(await screen.findByRole("img", { name: "B" })).toHaveAttribute("src", URL_B);

    resolveA(URL_A);
    await Promise.resolve();
    expect(screen.getByRole("img", { name: "B" })).toHaveAttribute("src", URL_B);
  });
});
