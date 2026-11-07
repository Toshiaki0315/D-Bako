import { describe, expect, it, vi } from "vitest";
import { LOAD_RETRY_DELAYS, retry } from "./retry";

describe("retry", () => {
  it("returns the result without waiting when the first attempt succeeds", async () => {
    const sleep = vi.fn().mockResolvedValue(undefined);
    const fn = vi.fn().mockResolvedValue("ok");
    await expect(retry(fn, [10, 20], sleep)).resolves.toBe("ok");
    expect(fn).toHaveBeenCalledTimes(1);
    expect(sleep).not.toHaveBeenCalled();
  });

  it("retries after each delay until it succeeds", async () => {
    const sleep = vi.fn().mockResolvedValue(undefined);
    const fn = vi.fn().mockRejectedValueOnce("e1").mockRejectedValueOnce("e2").mockResolvedValue("ok");
    await expect(retry(fn, [10, 20, 30], sleep)).resolves.toBe("ok");
    expect(fn).toHaveBeenCalledTimes(3);
    expect(sleep.mock.calls.map(([ms]) => ms)).toEqual([10, 20]);
  });

  it("throws the last error once every delay has been used", async () => {
    const sleep = vi.fn().mockResolvedValue(undefined);
    const fn = vi.fn().mockRejectedValueOnce("e1").mockRejectedValueOnce("e2").mockRejectedValue("last");
    await expect(retry(fn, [10, 20], sleep)).rejects.toBe("last");
    expect(fn).toHaveBeenCalledTimes(3);
    expect(sleep).toHaveBeenCalledTimes(2);
  });

  it("makes a single attempt when there are no delays", async () => {
    const fn = vi.fn().mockRejectedValue("nope");
    await expect(retry(fn, [])).rejects.toBe("nope");
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("waits for real with the default sleep", async () => {
    vi.useFakeTimers();
    const fn = vi.fn().mockRejectedValueOnce("e1").mockResolvedValue("ok");
    const result = retry(fn, [50]);
    await vi.advanceTimersByTimeAsync(50);
    await expect(result).resolves.toBe("ok");
    vi.useRealTimers();
  });
});

describe("LOAD_RETRY_DELAYS", () => {
  it("backs off and gives up within a few seconds", () => {
    expect(LOAD_RETRY_DELAYS.length).toBeGreaterThan(1);
    expect([...LOAD_RETRY_DELAYS].sort((a, b) => a - b)).toEqual(LOAD_RETRY_DELAYS);
    expect(LOAD_RETRY_DELAYS.reduce((a, b) => a + b, 0)).toBeLessThanOrEqual(10_000);
  });
});
