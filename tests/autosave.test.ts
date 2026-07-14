import { describe, expect, it, vi } from "vitest";
import { AutosaveQueue } from "../src/services/autosave";

describe("AutosaveQueue", () => {
  it("debounces changes and saves the latest value", async () => {
    vi.useFakeTimers();
    const save = vi.fn(async () => undefined);
    const queue = new AutosaveQueue(save, 500);
    queue.schedule("a");
    queue.schedule("ab");
    await vi.advanceTimersByTimeAsync(500);
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith("ab");
    vi.useRealTimers();
  });

  it("does not save in the middle of IME composition", async () => {
    vi.useFakeTimers();
    const save = vi.fn(async () => undefined);
    const queue = new AutosaveQueue(save, 500);
    queue.setComposing(true);
    queue.schedule("中");
    await vi.advanceTimersByTimeAsync(1000);
    expect(save).not.toHaveBeenCalled();
    queue.setComposing(false);
    await vi.advanceTimersByTimeAsync(500);
    expect(save).toHaveBeenCalledWith("中");
    vi.useRealTimers();
  });

  it("serializes overlapping saves", async () => {
    const order: string[] = [];
    let releaseFirst!: () => void;
    const firstGate = new Promise<void>((resolve) => { releaseFirst = resolve; });
    const queue = new AutosaveQueue<number>(async (value) => {
      order.push(`start-${value}`);
      if (value === 1) await firstGate;
      order.push(`end-${value}`);
    }, 500);
    queue.schedule(1);
    const first = queue.flush();
    queue.schedule(2);
    const second = queue.flush();
    await vi.waitFor(() => expect(order).toEqual(["start-1"]));
    releaseFirst();
    await Promise.all([first, second]);
    expect(order).toEqual(["start-1", "end-1", "start-2", "end-2"]);
  });

  it("continues saving after a previous failure", async () => {
    const save = vi.fn(async (value: string) => {
      if (value === "bad") throw new Error("disk busy");
    });
    const queue = new AutosaveQueue(save, 500);
    queue.schedule("bad");
    await expect(queue.flush()).rejects.toThrow("disk busy");
    queue.schedule("recovered");
    await expect(queue.flush()).resolves.toBeUndefined();
    expect(save).toHaveBeenNthCalledWith(2, "recovered");
  });
});
