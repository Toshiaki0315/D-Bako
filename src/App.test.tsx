import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
const unlistenMock = vi.fn();
let dropHandler: ((event: { payload: { paths?: string[] } }) => Promise<void>) | null;
let settingsHandler: (() => void) | null;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: never) => {
    if (name === "tauri://drop") dropHandler = handler;
    if (name === "open-settings") settingsHandler = handler;
    return unlistenMock;
  }),
}));

import App from "./App";

type AppItem = { path: string; name: string; kind?: "app" | "url"; icon?: string };

function makeState(maxPages = 3, filled: Record<string, AppItem> = {}) {
  const pages = Array.from({ length: maxPages }, (_, p) =>
    Array.from({ length: 8 }, (_, s) => filled[`${p}:${s}`] ?? null),
  );
  return { pages, max_pages: maxPages };
}

const foo: AppItem = { path: "/Applications/Foo.app", name: "Foo" };

function mockBackend(state: ReturnType<typeof makeState>) {
  invokeMock.mockImplementation(async (cmd: string) => {
    if (cmd === "get_state") return state;
    return undefined;
  });
}

async function renderApp(state = makeState(3, { "0:0": foo })) {
  mockBackend(state);
  render(<App />);
  await screen.findByText(/^1 \/ \d+$/);
}

async function drop(paths?: string[]) {
  await act(async () => {
    await dropHandler!({ payload: { paths } });
  });
}

beforeEach(() => {
  invokeMock.mockReset();
  unlistenMock.mockReset();
  dropHandler = null;
  settingsHandler = null;
});

describe("App", () => {
  it("shows a loading message until the state arrives", async () => {
    mockBackend(makeState());
    render(<App />);
    expect(screen.getByText("Loading...")).toBeInTheDocument();
    await screen.findByText("1 / 3");
  });

  it("renders 8 slots and the registered app name", async () => {
    await renderApp();
    expect(screen.getByRole("button", { name: "Foo" })).toBeInTheDocument();
    expect(document.querySelectorAll(".slot")).toHaveLength(8);
    expect(document.querySelectorAll(".slot.filled")).toHaveLength(1);
  });

  it("shows the app name next to the mouse pointer and follows it", async () => {
    await renderApp();
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();

    const slot = screen.getByRole("button", { name: "Foo" });
    fireEvent.mouseMove(slot, { clientX: 100, clientY: 50 });
    const tooltip = screen.getByRole("tooltip");
    expect(tooltip).toHaveTextContent("Foo");
    expect(tooltip).toHaveStyle({ left: "112px", top: "68px" });

    fireEvent.mouseMove(slot, { clientX: 130, clientY: 70 });
    expect(screen.getByRole("tooltip")).toHaveStyle({ left: "142px", top: "88px" });

    fireEvent.mouseLeave(slot);
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("flips the tooltip to the left of the pointer near the right edge", async () => {
    await renderApp();
    fireEvent.mouseMove(screen.getByRole("button", { name: "Foo" }), {
      clientX: window.innerWidth - 10,
      clientY: 50,
    });
    expect(screen.getByRole("tooltip")).toHaveStyle({ transform: "translateX(-100%)" });
  });

  it("shows no tooltip for an empty slot", async () => {
    await renderApp();
    fireEvent.mouseMove(document.querySelector(".slot:not(.filled)")!, { clientX: 10, clientY: 10 });
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("has no instruction text", async () => {
    await renderApp();
    expect(screen.queryByText(/ドラッグ/)).not.toBeInTheDocument();
  });

  it("provides a drag region so the window can be moved by its top edge", async () => {
    await renderApp();
    expect(document.querySelector("[data-tauri-drag-region]")).not.toBeNull();
  });

  it("puts icon-only pagination below the grid", async () => {
    await renderApp();
    const prev = screen.getByRole("button", { name: "Previous page" });
    const next = screen.getByRole("button", { name: "Next page" });
    expect(prev.textContent).toBe("");
    expect(next.textContent).toBe("");

    const grid = document.querySelector(".grid")!;
    for (const el of [prev, next, screen.getByText("1 / 3")]) {
      expect(grid.compareDocumentPosition(el) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    }
  });

  it("shows the extracted icon image in a filled slot", async () => {
    const state = makeState(3, { "0:0": foo });
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "get_state") return state;
      if (cmd === "get_app_icon") return "data:image/png;base64,AAAA";
      return undefined;
    });
    render(<App />);
    const img = await screen.findByRole("img", { name: "Foo" });
    expect(img).toHaveAttribute("src", "data:image/png;base64,AAAA");
  });

  it("launches the app when a filled slot is clicked", async () => {
    await renderApp();
    await userEvent.click(screen.getByRole("button", { name: "Foo" }));
    expect(invokeMock).toHaveBeenCalledWith("launch_app", { path: foo.path });
  });

  it("does nothing when an empty slot is clicked", async () => {
    await renderApp();
    await userEvent.click(screen.getByText("2"));
    expect(invokeMock).not.toHaveBeenCalledWith("launch_app", expect.anything());
  });

  it("navigates between pages and disables buttons at the bounds", async () => {
    await renderApp();
    const prev = screen.getByRole("button", { name: "Previous page" });
    const next = screen.getByRole("button", { name: "Next page" });
    expect(prev).toBeDisabled();

    await userEvent.click(next);
    expect(screen.getByText("2 / 3")).toBeInTheDocument();
    await userEvent.click(next);
    expect(screen.getByText("3 / 3")).toBeInTheDocument();
    expect(next).toBeDisabled();

    await userEvent.click(prev);
    expect(screen.getByText("2 / 3")).toBeInTheDocument();
  });

  it("registers a dropped .app on the current page and refreshes state", async () => {
    await renderApp();
    await userEvent.click(screen.getByRole("button", { name: "Next page" }));
    invokeMock.mockClear();

    await drop(["/tmp/readme.txt", "/Applications/Bar.app"]);

    expect(invokeMock).toHaveBeenCalledWith("add_app_to_first_empty", {
      pageIdx: 1,
      path: "/Applications/Bar.app",
    });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_state"));
  });

  it("ignores a drop that contains no .app bundle", async () => {
    await renderApp();
    invokeMock.mockClear();

    await drop(["/tmp/readme.txt"]);

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("shows the reason when registering fails, and clears it on success", async () => {
    await renderApp();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "add_app_to_first_empty") throw "No empty slot on this page";
      return makeState();
    });

    await drop(["/Applications/Bar.app"]);
    expect(await screen.findByRole("alert")).toHaveTextContent("No empty slot on this page");

    invokeMock.mockImplementation(async () => makeState());
    await drop(["/Applications/Bar.app"]);
    await waitFor(() => expect(screen.queryByRole("alert")).not.toBeInTheDocument());
  });

  it("ignores a drop without paths", async () => {
    await renderApp();
    invokeMock.mockClear();

    await drop(undefined);
    await drop([]);

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("unsubscribes from the drop event on unmount", async () => {
    mockBackend(makeState());
    const { unmount } = render(<App />);
    await screen.findByText("1 / 3");
    unmount();
    await waitFor(() => expect(unlistenMock).toHaveBeenCalled());
  });
});

const urlItem: AppItem = {
  path: "https://example.com/",
  name: "example.com",
  kind: "url",
  icon: "data:image/png;base64,AAAA",
};

function paste(text: string, target: Element = document.body) {
  fireEvent.paste(target, { clipboardData: { getData: () => text } });
}

describe("App URL shortcuts", () => {
  it("shows a URL shortcut with its stored favicon without extracting an icon", async () => {
    await renderApp(makeState(3, { "0:0": urlItem }));
    expect(screen.getByRole("img", { name: "example.com" })).toHaveAttribute("src", urlItem.icon);
    expect(invokeMock).not.toHaveBeenCalledWith("get_app_icon", expect.anything());
  });

  it("launches a URL shortcut with its URL", async () => {
    await renderApp(makeState(3, { "0:0": urlItem }));
    await userEvent.click(screen.getByRole("button", { name: "example.com" }));
    expect(invokeMock).toHaveBeenCalledWith("launch_app", { path: "https://example.com/" });
  });

  it("registers a pasted URL on the current page and refreshes state", async () => {
    await renderApp();
    await userEvent.click(screen.getByRole("button", { name: "Next page" }));
    invokeMock.mockClear();

    paste("  example.com\n");

    await waitFor(() =>
      expect(invokeMock).toHaveBeenCalledWith("add_url_to_first_empty", { pageIdx: 1, input: "example.com" }),
    );
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_state"));
  });

  it("ignores pasted text that does not look like a URL", async () => {
    await renderApp();
    invokeMock.mockClear();
    paste("just some words");
    paste("");
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("shows the reason when a pasted URL cannot be registered", async () => {
    await renderApp();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "add_url_to_first_empty") throw "No empty slot on this page";
      return makeState();
    });
    paste("example.com");
    expect(await screen.findByRole("alert")).toHaveTextContent("No empty slot on this page");
  });

  it("registers a dropped .webloc / .url file", async () => {
    await renderApp();
    invokeMock.mockClear();
    await drop(["/tmp/readme.txt", "/Users/me/Site.webloc"]);
    expect(invokeMock).toHaveBeenCalledWith("add_url_file_to_first_empty", {
      pageIdx: 0,
      path: "/Users/me/Site.webloc",
    });
    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("get_state"));
  });

  it("prefers a dropped .app over a shortcut file", async () => {
    await renderApp();
    invokeMock.mockClear();
    await drop(["/Users/me/Site.webloc", "/Applications/Bar.app"]);
    expect(invokeMock).toHaveBeenCalledWith("add_app_to_first_empty", { pageIdx: 0, path: "/Applications/Bar.app" });
    expect(invokeMock).not.toHaveBeenCalledWith("add_url_file_to_first_empty", expect.anything());
  });

  it("shows the reason when a dropped shortcut file is invalid", async () => {
    await renderApp();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "add_url_file_to_first_empty") throw "URL not found in webloc";
      return makeState();
    });
    await drop(["/Users/me/Broken.webloc"]);
    expect(await screen.findByRole("alert")).toHaveTextContent("URL not found in webloc");
  });
});

describe("App URL dialog", () => {
  const emptySlot = (i: number) => document.querySelectorAll(".slot")[i] as HTMLElement;

  it("opens from an empty slot and registers the URL into that slot", async () => {
    await renderApp();
    invokeMock.mockClear();
    await userEvent.click(emptySlot(3));

    const input = screen.getByRole("textbox", { name: "URL" });
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    await userEvent.type(input, "example.com{Enter}");

    expect(invokeMock).toHaveBeenCalledWith("add_url_to_slot", { pageIdx: 0, slotIdx: 3, input: "example.com" });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(invokeMock).toHaveBeenCalledWith("get_state");
  });

  it("does not open for a filled slot", async () => {
    await renderApp();
    await userEvent.click(screen.getByRole("button", { name: "Foo" }));
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("closes with Escape or a click on the backdrop without registering", async () => {
    await renderApp();
    invokeMock.mockClear();

    await userEvent.click(emptySlot(1));
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();

    await userEvent.click(emptySlot(1));
    await userEvent.click(document.querySelector(".backdrop")!);
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith("add_url_to_slot", expect.anything());
  });

  it("does not close when the dialog itself is clicked", async () => {
    await renderApp();
    await userEvent.click(emptySlot(1));
    await userEvent.click(screen.getByRole("dialog"));
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("ignores an empty submission", async () => {
    await renderApp();
    invokeMock.mockClear();
    await userEvent.click(emptySlot(1));
    await userEvent.type(screen.getByRole("textbox", { name: "URL" }), "   {Enter}");
    expect(invokeMock).not.toHaveBeenCalledWith("add_url_to_slot", expect.anything());
    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("stays open and shows the reason when registering fails", async () => {
    await renderApp();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "add_url_to_slot") throw "Invalid host";
      return makeState();
    });
    await userEvent.click(emptySlot(2));
    await userEvent.type(screen.getByRole("textbox", { name: "URL" }), "hello{Enter}");

    expect(await screen.findByText(/Invalid host/)).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "URL" })).toBeEnabled();
  });

  it("disables the input while the favicon is being fetched", async () => {
    await renderApp();
    let finish!: () => void;
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "add_url_to_slot") return new Promise<void>((r) => (finish = r));
      return makeState();
    });
    await userEvent.click(emptySlot(2));
    await userEvent.type(screen.getByRole("textbox", { name: "URL" }), "example.com{Enter}");
    expect(screen.getByRole("textbox", { name: "URL" })).toBeDisabled();

    await act(async () => finish());
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("does not treat a paste inside the dialog as a quick registration", async () => {
    await renderApp();
    invokeMock.mockClear();
    await userEvent.click(emptySlot(1));
    paste("example.com", screen.getByRole("textbox", { name: "URL" }));
    expect(invokeMock).not.toHaveBeenCalledWith("add_url_to_first_empty", expect.anything());
  });
});

describe("App settings dialog", () => {
  function mockSettings(alwaysOnTop: boolean, setAlwaysOnTop: (enabled: boolean) => Promise<void> = async () => {}) {
    invokeMock.mockImplementation(async (cmd: string, args?: { enabled: boolean }) => {
      if (cmd === "get_state") return makeState();
      if (cmd === "get_settings") return { always_on_top: alwaysOnTop };
      if (cmd === "set_always_on_top") return setAlwaysOnTop(args!.enabled);
      return undefined;
    });
  }

  async function openSettings() {
    await waitFor(() => expect(settingsHandler).not.toBeNull());
    act(() => settingsHandler!());
    return screen.findByRole("checkbox", { name: "常に前に表示" });
  }

  it("opens from the menu and shows the saved setting", async () => {
    await renderApp();
    mockSettings(true);
    expect(screen.queryByRole("dialog", { name: "設定" })).not.toBeInTheDocument();

    const checkbox = await openSettings();
    expect(screen.getByRole("dialog", { name: "設定" })).toBeInTheDocument();
    expect(checkbox).toBeChecked();
  });

  it("turns always-on-top on and off", async () => {
    await renderApp();
    mockSettings(false);
    const checkbox = await openSettings();
    expect(checkbox).not.toBeChecked();

    await userEvent.click(checkbox);
    expect(invokeMock).toHaveBeenCalledWith("set_always_on_top", { enabled: true });
    expect(checkbox).toBeChecked();

    await userEvent.click(checkbox);
    expect(invokeMock).toHaveBeenCalledWith("set_always_on_top", { enabled: false });
    expect(checkbox).not.toBeChecked();
  });

  it("keeps the previous value and shows the reason when the change fails", async () => {
    await renderApp();
    mockSettings(false, async () => { throw "write denied"; });
    const checkbox = await openSettings();

    await userEvent.click(checkbox);
    expect(await screen.findByText(/write denied/)).toBeInTheDocument();
    expect(checkbox).not.toBeChecked();
  });

  it("closes with the close button, Escape or a click on the backdrop", async () => {
    await renderApp();
    mockSettings(false);

    await openSettings();
    await userEvent.click(screen.getByRole("button", { name: "閉じる" }));
    expect(screen.queryByRole("dialog", { name: "設定" })).not.toBeInTheDocument();

    await openSettings();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog", { name: "設定" })).not.toBeInTheDocument();

    await openSettings();
    await userEvent.click(document.querySelector(".backdrop")!);
    expect(screen.queryByRole("dialog", { name: "設定" })).not.toBeInTheDocument();
  });

  it("does not close when the dialog itself is clicked", async () => {
    await renderApp();
    mockSettings(false);
    await openSettings();
    await userEvent.click(screen.getByRole("dialog", { name: "設定" }));
    expect(screen.getByRole("dialog", { name: "設定" })).toBeInTheDocument();
  });
});

describe("App rename", () => {
  const filled = () => screen.getByRole("button", { name: "Foo" });

  async function openRenameDialog() {
    fireEvent.contextMenu(filled(), { clientX: 40, clientY: 60 });
    await userEvent.click(screen.getByRole("menuitem", { name: "名前を変更…" }));
    return screen.getByRole("textbox", { name: "名前" });
  }

  it("opens a context menu with a rename item on a registered item and hides the tooltip", async () => {
    await renderApp();
    fireEvent.mouseMove(filled(), { clientX: 40, clientY: 60 });
    const event = fireEvent.contextMenu(filled(), { clientX: 40, clientY: 60 });

    expect(event).toBe(false); // WebView 標準のメニューは出さない
    expect(screen.getByRole("menu")).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "名前を変更…" })).toBeInTheDocument();
    expect(screen.queryByRole("tooltip")).not.toBeInTheDocument();
  });

  it("does not open a context menu on an empty slot", async () => {
    await renderApp();
    fireEvent.contextMenu(document.querySelectorAll(".slot")[1]);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
  });

  it("closes the context menu with Escape or a click elsewhere, without opening the dialog", async () => {
    await renderApp();
    fireEvent.contextMenu(filled());
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    fireEvent.contextMenu(filled());
    await userEvent.click(document.querySelector(".context-menu-layer")!);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();

    fireEvent.contextMenu(filled());
    fireEvent.contextMenu(document.querySelector(".context-menu-layer")!);
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("renames the item with the name entered in the dialog", async () => {
    await renderApp();
    const input = await openRenameDialog();
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "名前を変更" })).toBeInTheDocument();
    expect(input).toHaveValue("Foo");

    invokeMock.mockClear();
    await userEvent.clear(input);
    await userEvent.type(input, "Googleカレンダー{Enter}");

    expect(invokeMock).toHaveBeenCalledWith("rename_app", { pageIdx: 0, slotIdx: 0, name: "Googleカレンダー" });
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(invokeMock).toHaveBeenCalledWith("get_state");
  });

  it("renames an item on the current page", async () => {
    await renderApp(makeState(3, { "1:4": foo }));
    await userEvent.click(screen.getByRole("button", { name: "Next page" }));
    const input = await openRenameDialog();
    await userEvent.type(input, "2{Enter}");
    expect(invokeMock).toHaveBeenCalledWith("rename_app", { pageIdx: 1, slotIdx: 4, name: "Foo2" });
  });

  it("does not launch the item when it is right-clicked", async () => {
    await renderApp();
    await openRenameDialog();
    expect(invokeMock).not.toHaveBeenCalledWith("launch_app", expect.anything());
  });

  it("ignores a blank name", async () => {
    await renderApp();
    const input = await openRenameDialog();
    invokeMock.mockClear();
    await userEvent.clear(input);
    await userEvent.type(input, "   {Enter}");
    expect(invokeMock).not.toHaveBeenCalledWith("rename_app", expect.anything());
    expect(screen.getByRole("dialog", { name: "名前を変更" })).toBeInTheDocument();
  });

  it("stays open and shows the reason when renaming fails", async () => {
    await renderApp();
    const input = await openRenameDialog();
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "rename_app") throw "Name is too long";
      return makeState();
    });
    await userEvent.type(input, "x{Enter}");
    expect(await screen.findByText(/Name is too long/)).toBeInTheDocument();
    expect(screen.getByRole("dialog", { name: "名前を変更" })).toBeInTheDocument();
  });

  it("closes the dialog with Escape without renaming", async () => {
    await renderApp();
    await openRenameDialog();
    await userEvent.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalledWith("rename_app", expect.anything());
  });
});
