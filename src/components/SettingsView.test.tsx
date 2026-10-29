import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { SettingsView } from "./SettingsView";

function mockSettings(
  alwaysOnTop: boolean,
  setAlwaysOnTop: (enabled: boolean) => Promise<void> = async () => {},
  launchAtLogin = false,
  setLaunchAtLogin: (enabled: boolean) => Promise<void> = async () => {},
) {
  invokeMock.mockImplementation(async (cmd: string, args?: { enabled: boolean }) => {
    if (cmd === "get_settings") return { always_on_top: alwaysOnTop, launch_at_login: launchAtLogin };
    if (cmd === "set_always_on_top") return setAlwaysOnTop(args!.enabled);
    if (cmd === "set_launch_at_login") return setLaunchAtLogin(args!.enabled);
    return undefined;
  });
}

async function renderSettings() {
  render(<SettingsView />);
  return screen.findByRole("checkbox", { name: "常に前に表示" });
}

beforeEach(() => invokeMock.mockReset());

describe("SettingsView", () => {
  it("shows the saved setting", async () => {
    mockSettings(true);
    expect(await renderSettings()).toBeChecked();
  });

  it("shows nothing until the settings arrive", async () => {
    let resolve!: (s: { always_on_top: boolean; launch_at_login: boolean }) => void;
    invokeMock.mockReturnValue(new Promise((r) => (resolve = r)));
    render(<SettingsView />);
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();

    resolve({ always_on_top: false, launch_at_login: false });
    expect(await screen.findByRole("checkbox", { name: "常に前に表示" })).not.toBeChecked();
  });

  it("turns always-on-top on and off", async () => {
    mockSettings(false);
    const checkbox = await renderSettings();
    expect(checkbox).not.toBeChecked();

    await userEvent.click(checkbox);
    expect(invokeMock).toHaveBeenCalledWith("set_always_on_top", { enabled: true });
    expect(checkbox).toBeChecked();

    await userEvent.click(checkbox);
    expect(invokeMock).toHaveBeenCalledWith("set_always_on_top", { enabled: false });
    expect(checkbox).not.toBeChecked();
  });

  it("toggles when the label text is clicked", async () => {
    mockSettings(false);
    const checkbox = await renderSettings();
    await userEvent.click(screen.getByText("常に前に表示"));
    expect(checkbox).toBeChecked();
  });

  it("keeps the previous value and shows the reason when the change fails", async () => {
    mockSettings(false, async () => { throw "write denied"; });
    const checkbox = await renderSettings();

    await userEvent.click(checkbox);
    expect(await screen.findByRole("alert")).toHaveTextContent("変更できません: write denied");
    expect(checkbox).not.toBeChecked();
  });

  it("clears the previous error after a successful change", async () => {
    let fail = true;
    mockSettings(false, async () => { if (fail) throw "write denied"; });
    const checkbox = await renderSettings();
    await userEvent.click(checkbox);
    await screen.findByRole("alert");

    fail = false;
    await userEvent.click(checkbox);
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  describe("launch at login", () => {
    const loginCheckbox = () => screen.getByRole("checkbox", { name: "起動時に実行する" });

    it("shows whether the app is registered to launch at login", async () => {
      mockSettings(false, undefined, true);
      await renderSettings();
      expect(loginCheckbox()).toBeChecked();
    });

    it("turns launch at login on and off without touching always-on-top", async () => {
      mockSettings(false);
      const onTop = await renderSettings();

      await userEvent.click(loginCheckbox());
      expect(invokeMock).toHaveBeenCalledWith("set_launch_at_login", { enabled: true });
      expect(loginCheckbox()).toBeChecked();

      await userEvent.click(loginCheckbox());
      expect(invokeMock).toHaveBeenCalledWith("set_launch_at_login", { enabled: false });
      expect(loginCheckbox()).not.toBeChecked();

      expect(invokeMock).not.toHaveBeenCalledWith("set_always_on_top", expect.anything());
      expect(onTop).not.toBeChecked();
    });

    it("keeps the previous value and shows the reason when registration fails", async () => {
      mockSettings(false, undefined, false, async () => { throw "launchctl failed"; });
      await renderSettings();

      await userEvent.click(loginCheckbox());
      expect(await screen.findByRole("alert")).toHaveTextContent("変更できません: launchctl failed");
      expect(loginCheckbox()).not.toBeChecked();
    });
  });
});

describe("SettingsView layout", () => {
  type Layout = { cols: number; rows: number; pages: number };

  function mockLayout(layout: Layout, setLayout: (l: Layout) => Promise<void> = async () => {}) {
    invokeMock.mockImplementation(async (cmd: string, args?: Layout) => {
      if (cmd === "get_settings") return { always_on_top: false, launch_at_login: false };
      if (cmd === "get_layout") return layout;
      if (cmd === "set_layout") return setLayout(args!);
    });
  }

  it("shows the current grid size and page count", async () => {
    mockLayout({ cols: 4, rows: 2, pages: 10 });
    render(<SettingsView />);
    expect(await screen.findByRole("combobox", { name: "横の数" })).toHaveValue("4");
    expect(screen.getByRole("combobox", { name: "縦の数" })).toHaveValue("2");
    expect(screen.getByRole("combobox", { name: "ページ数" })).toHaveValue("10");
  });

  it("groups the general settings and the grid settings", async () => {
    mockLayout({ cols: 4, rows: 2, pages: 10 });
    render(<SettingsView />);
    const general = await screen.findByRole("group", { name: "一般" });
    expect(within(general).getByRole("checkbox", { name: "常に前に表示" })).toBeInTheDocument();
    expect(within(general).getByRole("checkbox", { name: "起動時に実行する" })).toBeInTheDocument();
    const grid = screen.getByRole("group", { name: "グリッド" });
    expect(within(grid).getAllByRole("combobox")).toHaveLength(3);
  });

  it("offers 1-8 columns, 1-6 rows and 1-10 pages", async () => {
    mockLayout({ cols: 4, rows: 2, pages: 10 });
    render(<SettingsView />);
    const values = (name: string) =>
      Array.from(screen.getByRole("combobox", { name }).querySelectorAll("option"), (o) => o.value);
    await screen.findByRole("combobox", { name: "横の数" });
    expect(values("横の数")).toEqual(["1", "2", "3", "4", "5", "6", "7", "8"]);
    expect(values("縦の数")).toEqual(["1", "2", "3", "4", "5", "6"]);
    expect(values("ページ数")).toEqual(["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"]);
  });

  it("applies a new value right away", async () => {
    mockLayout({ cols: 4, rows: 2, pages: 10 });
    render(<SettingsView />);
    await userEvent.selectOptions(await screen.findByRole("combobox", { name: "横の数" }), "6");
    expect(invokeMock).toHaveBeenCalledWith("set_layout", { cols: 6, rows: 2, pages: 10 });
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "ページ数" }), "3");
    expect(invokeMock).toHaveBeenCalledWith("set_layout", { cols: 6, rows: 2, pages: 3 });
    expect(screen.getByRole("combobox", { name: "ページ数" })).toHaveValue("3");
  });

  it("goes back and shows the reason when the items do not fit", async () => {
    mockLayout({ cols: 4, rows: 2, pages: 10 }, async () => { throw "Not enough slots for the registered items"; });
    render(<SettingsView />);
    const rows = await screen.findByRole("combobox", { name: "縦の数" });
    await userEvent.selectOptions(rows, "1");
    expect(await screen.findByRole("alert")).toHaveTextContent("変更できません: Not enough slots for the registered items");
    expect(rows).toHaveValue("2");
  });
});
