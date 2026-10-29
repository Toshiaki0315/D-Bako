import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { SettingsView } from "./SettingsView";

function mockSettings(alwaysOnTop: boolean, setAlwaysOnTop: (enabled: boolean) => Promise<void> = async () => {}) {
  invokeMock.mockImplementation(async (cmd: string, args?: { enabled: boolean }) => {
    if (cmd === "get_settings") return { always_on_top: alwaysOnTop };
    if (cmd === "set_always_on_top") return setAlwaysOnTop(args!.enabled);
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
    let resolve!: (s: { always_on_top: boolean }) => void;
    invokeMock.mockReturnValue(new Promise((r) => (resolve = r)));
    render(<SettingsView />);
    expect(screen.queryByRole("checkbox")).not.toBeInTheDocument();

    resolve({ always_on_top: false });
    expect(await screen.findByRole("checkbox")).not.toBeChecked();
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
});
