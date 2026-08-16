import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriStore } from "@/hooks/useTauriStore";
import type {
  PerformanceCustomLayout,
  PerformanceView,
} from "../types/performanceLayout";
import { usePerformanceLayout } from "./usePerformanceLayout";

const setView = vi.fn();
const setCustomLayout = vi.fn();
const setColumns = vi.fn();
const setCompactExpanded = vi.fn();
let columns: unknown = 1;
let compactExpanded: unknown = false;

let view: PerformanceView = "panels";
let customLayout: PerformanceCustomLayout = {
  order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
  visible: ["usageGraphs", "processTable"],
};

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi.fn(),
}));

describe("usePerformanceLayout", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    view = "panels";
    customLayout = {
      order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
      visible: ["usageGraphs", "processTable"],
    };
    columns = 1;
    compactExpanded = false;
    setView.mockResolvedValue(undefined);
    setCustomLayout.mockResolvedValue(undefined);
    setColumns.mockResolvedValue(undefined);
    setCompactExpanded.mockResolvedValue(undefined);
    vi.mocked(useTauriStore).mockImplementation((key) => {
      if (key === "performanceLayoutPreset") {
        return [view, setView, false] as never;
      }
      if (key === "performancePanelColumns") {
        return [columns, setColumns, false] as never;
      }
      if (key === "performanceCompactExpanded") {
        return [compactExpanded, setCompactExpanded, false] as never;
      }
      return [customLayout, setCustomLayout, false] as never;
    });
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("persists view selection in UI-local store", async () => {
    const { result } = renderHook(() => usePerformanceLayout());

    await act(async () => result.current.setView("monitor"));

    expect(setView).toHaveBeenCalledWith("monitor");
  });

  it("persists the panel column count and repairs unusable values", async () => {
    columns = 7;
    const { result } = renderHook(() => usePerformanceLayout());

    expect(result.current.columns).toBe(1);

    await act(async () => result.current.setColumns(2));

    expect(setColumns).toHaveBeenCalledWith(2);
  });

  it("persists the mini-monitor choice and treats a non-true value as collapsed", async () => {
    compactExpanded = true;
    const { result, rerender } = renderHook(() => usePerformanceLayout());

    expect(result.current.compactExpanded).toBe(true);

    await act(async () => result.current.setCompactExpanded(false));

    expect(setCompactExpanded).toHaveBeenCalledWith(false);

    // A store that has not resolved yet must not read as expanded.
    compactExpanded = null;
    rerender();

    expect(result.current.compactExpanded).toBe(false);
  });

  it("stays pending until the mini-monitor store resolves", () => {
    vi.mocked(useTauriStore).mockImplementation((key) =>
      key === "performanceCompactExpanded"
        ? ([null, setCompactExpanded, true] as never)
        : ([view, setView, false] as never),
    );

    const { result } = renderHook(() => usePerformanceLayout());

    // Otherwise the screen renders un-expanded for a frame and then snaps
    // into the mini monitor once this store resolves.
    expect(result.current.isPending).toBe(true);
  });

  it("normalizes a stored legacy preset onto a view", () => {
    view = "detailed" as PerformanceView;
    const { result } = renderHook(() => usePerformanceLayout());

    expect(result.current.view).toBe("panels");
  });

  it("reorders panels without changing visibility", async () => {
    const { result } = renderHook(() => usePerformanceLayout());

    await act(async () => {
      result.current.handlePanelDragEnd({
        active: { id: "processTable" },
        over: { id: "usageGraphs" },
      } as never);
      await Promise.resolve();
    });

    expect(setCustomLayout).toHaveBeenCalledWith({
      order: ["processTable", "usageGraphs", "perCore", "motherboardSensors"],
      visible: ["usageGraphs", "processTable"],
    });
  });

  it("allows hiding the final visible panel because instruments stay mounted", async () => {
    customLayout = {
      order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
      visible: ["usageGraphs"],
    };
    const { result } = renderHook(() => usePerformanceLayout());

    let changed = false;
    await act(async () => {
      changed = await result.current.togglePanel("usageGraphs");
    });

    expect(changed).toBe(true);
    expect(setCustomLayout).toHaveBeenCalledWith({
      order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
      visible: [],
    });
  });

  it("serializes rapid panel mutations against the latest layout", async () => {
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
      firstMutation = result.current.togglePanel("usageGraphs");
      secondMutation = result.current.togglePanel("processTable");
    });

    await waitFor(() => expect(setCustomLayout).toHaveBeenCalledOnce());
    expect(setCustomLayout).toHaveBeenLastCalledWith({
      order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
      visible: ["processTable"],
    });

    rerender();

    await act(async () => {
      resolveFirstWrite?.();
      await Promise.all([firstMutation, secondMutation]);
    });

    expect(setCustomLayout).toHaveBeenCalledTimes(2);
    expect(setCustomLayout).toHaveBeenLastCalledWith({
      order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
      visible: [],
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
      changed = await result.current.togglePanel("usageGraphs");
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
        over: { id: "usageGraphs" },
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
