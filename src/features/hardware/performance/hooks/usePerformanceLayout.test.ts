import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriStore } from "@/hooks/useTauriStore";
import type {
  PerformanceCustomLayout,
  PerformanceLayoutPreset,
} from "../types/performanceLayout";
import { usePerformanceLayout } from "./usePerformanceLayout";

const setPreset = vi.fn();
const setCustomLayout = vi.fn();

let preset: PerformanceLayoutPreset = "detailed";
let customLayout: PerformanceCustomLayout = {
  order: ["currentValues", "usageGraphs", "processTable"],
  visible: ["currentValues", "usageGraphs", "processTable"],
};

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi.fn(),
}));

describe("usePerformanceLayout", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    preset = "detailed";
    customLayout = {
      order: ["currentValues", "usageGraphs", "processTable"],
      visible: ["currentValues", "usageGraphs", "processTable"],
    };
    setPreset.mockResolvedValue(undefined);
    setCustomLayout.mockResolvedValue(undefined);
    vi.mocked(useTauriStore).mockImplementation((key) =>
      key === "performanceLayoutPreset"
        ? ([preset, setPreset, false] as never)
        : ([customLayout, setCustomLayout, false] as never),
    );
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("persists preset selection in UI-local store", async () => {
    const { result } = renderHook(() => usePerformanceLayout());

    await act(async () => result.current.setPreset("monitor"));

    expect(setPreset).toHaveBeenCalledWith("monitor");
  });

  it("reorders Custom panels without changing visibility", async () => {
    const { result } = renderHook(() => usePerformanceLayout());

    await act(async () => {
      result.current.handlePanelDragEnd({
        active: { id: "processTable" },
        over: { id: "currentValues" },
      } as never);
      await Promise.resolve();
    });

    expect(setCustomLayout).toHaveBeenCalledWith({
      order: ["processTable", "currentValues", "usageGraphs"],
      visible: ["currentValues", "usageGraphs", "processTable"],
    });
  });

  it("prevents hiding the final visible Custom panel", async () => {
    customLayout = {
      order: ["currentValues", "usageGraphs", "processTable"],
      visible: ["currentValues"],
    };
    const { result } = renderHook(() => usePerformanceLayout());

    let changed = true;
    await act(async () => {
      changed = await result.current.togglePanel("currentValues");
    });

    expect(changed).toBe(false);
    expect(setCustomLayout).not.toHaveBeenCalled();
  });

  it("serializes rapid Custom mutations against the latest layout", async () => {
    let resolveFirstWrite: (() => void) | undefined;
    setCustomLayout
      .mockImplementationOnce(
        () =>
          new Promise<void>((resolve) => {
            resolveFirstWrite = resolve;
          }),
      )
      .mockResolvedValue(undefined);
    const { result, rerender } = renderHook(() => usePerformanceLayout());

    let firstMutation: Promise<boolean> | undefined;
    let secondMutation: Promise<boolean> | undefined;
    act(() => {
      firstMutation = result.current.togglePanel("currentValues");
      secondMutation = result.current.togglePanel("usageGraphs");
    });

    await waitFor(() => expect(setCustomLayout).toHaveBeenCalledOnce());
    expect(setCustomLayout).toHaveBeenLastCalledWith({
      order: ["currentValues", "usageGraphs", "processTable"],
      visible: ["usageGraphs", "processTable"],
    });

    rerender();

    await act(async () => {
      resolveFirstWrite?.();
      await Promise.all([firstMutation, secondMutation]);
    });

    expect(setCustomLayout).toHaveBeenCalledTimes(2);
    expect(setCustomLayout).toHaveBeenLastCalledWith({
      order: ["currentValues", "usageGraphs", "processTable"],
      visible: ["processTable"],
    });
  });

  it("handles rejected panel visibility writes without leaking the rejection", async () => {
    const persistenceError = new Error("store unavailable");
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);
    setCustomLayout.mockRejectedValueOnce(persistenceError);
    const { result } = renderHook(() => usePerformanceLayout());

    let changed = true;
    await act(async () => {
      changed = await result.current.togglePanel("currentValues");
    });

    expect(changed).toBe(false);
    expect(consoleError).toHaveBeenCalledWith(
      "Failed to persist custom Performance layout:",
      persistenceError,
    );
  });

  it("handles rejected panel-order writes without leaking the rejection", async () => {
    const persistenceError = new Error("store unavailable");
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);
    setCustomLayout.mockRejectedValueOnce(persistenceError);
    const { result } = renderHook(() => usePerformanceLayout());

    act(() => {
      result.current.handlePanelDragEnd({
        active: { id: "processTable" },
        over: { id: "currentValues" },
      } as never);
    });

    await waitFor(() =>
      expect(consoleError).toHaveBeenCalledWith(
        "Failed to persist custom Performance layout:",
        persistenceError,
      ),
    );
  });
});
