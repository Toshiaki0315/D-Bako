import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
const unlistenMock = vi.fn();
let dropHandler: ((event: { payload: { paths?: string[] } }) => Promise<void>) | null;

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (_name: string, handler: typeof dropHandler) => {
    dropHandler = handler;
    return unlistenMock;
  }),
}));

import App from "./App";

type AppItem = { path: string; name: string };

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
    expect(screen.getByText("Foo")).toBeInTheDocument();
    expect(document.querySelectorAll(".slot")).toHaveLength(8);
    expect(document.querySelectorAll(".slot.filled")).toHaveLength(1);
  });

  it("shows the app name as a tooltip on the filled slot only", async () => {
    await renderApp();
    const tooltips = screen.getAllByRole("tooltip");
    expect(tooltips).toHaveLength(1);
    expect(tooltips[0]).toHaveTextContent("Foo");
    expect(document.querySelector(".slot.filled")).toContainElement(tooltips[0]);
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
    await userEvent.click(screen.getByText("Foo"));
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
