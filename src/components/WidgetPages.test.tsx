import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const invokeMock = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => invokeMock(...args),
}));

import { ClockPage, PomodoroPage, TimerPage } from "./WidgetPages";

const MIN = 60_000;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2026, 10, 9, 7, 5, 3));
  invokeMock.mockReset();
  invokeMock.mockResolvedValue(undefined);
});
afterEach(() => vi.useRealTimers());

const advance = (ms: number) => act(() => { vi.advanceTimersByTime(ms); });
const click = (name: string) => fireEvent.click(screen.getByRole("button", { name }));
const display = () => screen.getByRole("timer");

describe("ClockPage", () => {
  it("shows the current time and date and keeps ticking", () => {
    render(<ClockPage />);
    expect(display()).toHaveTextContent("07:0503");
    expect(screen.getByText("2026年11月9日 (月)")).toBeInTheDocument();
    advance(1000);
    expect(display()).toHaveTextContent("07:0504");
  });
});

describe("TimerPage", () => {
  it("starts empty and cannot start until time is added", () => {
    render(<TimerPage />);
    expect(display()).toHaveTextContent("00:00");
    expect(screen.getByRole("button", { name: "開始" })).toBeDisabled();
  });

  it("adds time with the preset buttons", () => {
    render(<TimerPage />);
    click("+1分");
    click("+5分");
    click("+10分");
    expect(display()).toHaveTextContent("16:00");
  });

  it("shows the set time right after starting between screen updates", () => {
    render(<TimerPage />);
    click("+10分");
    advance(200); // 次の画面の更新 (250ms ごと) より前に開始する
    click("開始");
    expect(display()).toHaveTextContent("10:00");
    advance(1100); // 開始から 1 秒過ぎた後の最初の更新
    expect(display()).toHaveTextContent("09:59");
  });

  it("counts down, pauses and resets", () => {
    render(<TimerPage />);
    click("+5分");
    click("開始");
    advance(MIN);
    expect(display()).toHaveTextContent("04:00");

    click("一時停止");
    advance(MIN);
    expect(display()).toHaveTextContent("04:00");

    click("リセット");
    expect(display()).toHaveTextContent("05:00");
    expect(screen.getByRole("button", { name: "開始" })).toBeEnabled();
  });

  it("clears the time with reset when it is already at the set time", () => {
    render(<TimerPage />);
    click("+5分");
    click("リセット");
    expect(display()).toHaveTextContent("00:00");
  });

  it("plays the alert, says so and goes back to the set time when it ends", () => {
    render(<TimerPage />);
    click("+1分");
    click("開始");
    advance(MIN);
    expect(invokeMock).toHaveBeenCalledWith("play_alert");
    expect(screen.getByRole("status")).toHaveTextContent("時間になりました");
    expect(display()).toHaveTextContent("01:00");
    expect(screen.getByRole("button", { name: "開始" })).toBeEnabled();

    click("開始");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("does not break when the alert cannot be played", () => {
    invokeMock.mockRejectedValue("afplay missing");
    render(<TimerPage />);
    click("+1分");
    click("開始");
    advance(MIN);
    expect(screen.getByRole("status")).toHaveTextContent("時間になりました");
  });
});

describe("PomodoroPage", () => {
  it("starts with 25 minutes of work", () => {
    render(<PomodoroPage />);
    expect(screen.getByText("作業")).toBeInTheDocument();
    expect(display()).toHaveTextContent("25:00");
    expect(screen.getByText("完了 0 回")).toBeInTheDocument();
  });

  it("moves to a 5 minute break with the alert when the work ends", () => {
    render(<PomodoroPage />);
    click("開始");
    advance(25 * MIN);
    expect(invokeMock).toHaveBeenCalledWith("play_alert");
    expect(screen.getByText("休憩")).toBeInTheDocument();
    expect(display()).toHaveTextContent("05:00");
    expect(screen.getByText("完了 1 回")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("休憩しましょう");

    // 休憩は自分で始める
    advance(5 * MIN);
    expect(display()).toHaveTextContent("05:00");
    click("開始");
    advance(5 * MIN);
    expect(screen.getByText("作業")).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("作業に戻りましょう");
    expect(screen.getByText("完了 1 回")).toBeInTheDocument();
  });

  it("pauses and resets the current phase", () => {
    render(<PomodoroPage />);
    click("開始");
    advance(MIN);
    click("一時停止");
    advance(MIN);
    expect(display()).toHaveTextContent("24:00");
    click("リセット");
    expect(display()).toHaveTextContent("25:00");
  });

  it("skips to the next phase without the alert", () => {
    render(<PomodoroPage />);
    click("スキップ");
    expect(screen.getByText("休憩")).toBeInTheDocument();
    expect(screen.getByText("完了 0 回")).toBeInTheDocument();
    expect(invokeMock).not.toHaveBeenCalled();
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });
});
