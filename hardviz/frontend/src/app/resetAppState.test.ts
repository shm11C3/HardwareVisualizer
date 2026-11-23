import { beforeEach, describe, expect, it, vi } from "vitest";

const clearTauriStore = vi.fn();

vi.mock("@/lib/tauriStore", () => ({
  clearTauriStore: () => clearTauriStore(),
}));

import { resetAppState } from "./resetAppState";

describe("resetAppState", () => {
  beforeEach(() => {
    clearTauriStore.mockReset();
    vi.spyOn(console, "error").mockImplementation(() => {});
  });

  it("clears the Tauri Store before restarting the app state", async () => {
    const order: string[] = [];
    clearTauriStore.mockImplementation(async () => {
      order.push("clear");
    });
    await resetAppState(() => order.push("restart"));
    expect(order).toEqual(["clear", "restart"]);
  });

  it("still restarts the app state, and logs, when clearing fails", async () => {
    const failure = new Error("disk");
    clearTauriStore.mockRejectedValue(failure);
    const restart = vi.fn();
    await resetAppState(restart);
    expect(console.error).toHaveBeenCalledWith(
      "Failed to reset app state:",
      failure,
    );
    expect(restart).toHaveBeenCalledOnce();
  });
});
