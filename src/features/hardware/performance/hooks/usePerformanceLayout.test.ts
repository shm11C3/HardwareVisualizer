import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriStore } from "@/hooks/useTauriStore";
import type {
  PerformanceCustomLayout,
  PerformanceMonitorPowerMode,
  PerformanceView,
} from "../types/performanceLayout";
import { usePerformanceLayout } from "./usePerformanceLayout";

const setView = vi.fn();
const setCustomLayout = vi.fn();
const setColumns = vi.fn();
const setCompactExpanded = vi.fn();
const setMonitorPowerMode = vi.fn();
let columns: unknown = 1;
let compactExpanded: unknown = false;
let monitorPowerMode: unknown = "current" satisfies PerformanceMonitorPowerMode;

let view: PerformanceView = "panels";
let customLayout: PerformanceCustomLayout = {
  order: [
    "usageGraphs",
    "processTable",
    "perCore",
    "motherboardSensors",
    "power",
  ],
  visible: ["usageGraphs", "processTable", "power"],
};

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi.fn(),
}));

describe("usePerformanceLayout", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    view = "panels";
    customLayout = {
      order: [
        "usageGraphs",
        "processTable",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["usageGraphs", "processTable", "power"],
    };
    columns = 1;
    compactExpanded = false;
    monitorPowerMode = "current";
    setView.mockResolvedValue(undefined);
    setCustomLayout.mockResolvedValue(undefined);
    setColumns.mockResolvedValue(undefined);
    setCompactExpanded.mockResolvedValue(undefined);
    setMonitorPowerMode.mockResolvedValue(undefined);
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
      if (key === "performanceMonitorPowerMode") {
        return [monitorPowerMode, setMonitorPowerMode, false] as never;
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

  it("persists the Monitor Power Draw mode and repairs unknown values", async () => {
    monitorPowerMode = "overlay";
    const { result } = renderHook(() => usePerformanceLayout());

    expect(result.current.monitorPowerMode).toBe("current");
    await waitFor(() =>
      expect(setMonitorPowerMode).toHaveBeenCalledWith("current"),
    );

    await act(async () => result.current.setMonitorPowerMode("graph"));

    expect(setMonitorPowerMode).toHaveBeenLastCalledWith("graph");
  });

  it("reports a rejected Monitor Power Draw mode repair", async () => {
    const persistenceError = new Error("store unavailable");
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);
    monitorPowerMode = "overlay";
    setMonitorPowerMode.mockRejectedValueOnce(persistenceError);

    renderHook(() => usePerformanceLayout());

    await waitFor(() =>
      expect(consoleError).toHaveBeenCalledWith(
        "Failed to persist Performance Monitor Power Draw mode:",
        persistenceError,
      ),
    );
  });

  it("reports a rejected user-selected Monitor Power Draw mode", async () => {
    const persistenceError = new Error("store unavailable");
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);
    setMonitorPowerMode.mockRejectedValueOnce(persistenceError);
    const { result } = renderHook(() => usePerformanceLayout());

    await act(async () => result.current.setMonitorPowerMode("graph"));

    expect(consoleError).toHaveBeenCalledWith(
      "Failed to persist Performance Monitor Power Draw mode:",
      persistenceError,
    );
  });

  it("keeps the latest Monitor Power Draw mode after delayed writes", async () => {
    let resolveGraphWrite: (() => void) | undefined;
    setMonitorPowerMode
      .mockImplementationOnce(
        (nextMode: PerformanceMonitorPowerMode) =>
          new Promise<void>((resolve) => {
            resolveGraphWrite = () => {
              monitorPowerMode = nextMode;
              resolve();
            };
          }),
      )
      .mockImplementationOnce(async (nextMode: PerformanceMonitorPowerMode) => {
        monitorPowerMode = nextMode;
      });
    const { result, rerender } = renderHook(() => usePerformanceLayout());

    let graphWrite: Promise<void> | undefined;
    let currentWrite: Promise<void> | undefined;
    act(() => {
      graphWrite = result.current.setMonitorPowerMode("graph");
      currentWrite = result.current.setMonitorPowerMode("current");
    });

    await waitFor(() => expect(setMonitorPowerMode).toHaveBeenCalledOnce());
    expect(setMonitorPowerMode).toHaveBeenLastCalledWith("graph");

    await act(async () => {
      resolveGraphWrite?.();
      await Promise.all([graphWrite, currentWrite]);
    });
    rerender();

    expect(setMonitorPowerMode).toHaveBeenCalledTimes(2);
    expect(setMonitorPowerMode).toHaveBeenLastCalledWith("current");
    expect(result.current.monitorPowerMode).toBe("current");
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
    vi.mocked(useTauriStore).mockImplementation((key) => {
      if (key === "performanceLayoutPreset") {
        return [view, setView, false] as never;
      }
      if (key === "performanceCustomLayout") {
        return [customLayout, setCustomLayout, false] as never;
      }
      if (key === "performancePanelColumns") {
        return [columns, setColumns, false] as never;
      }
      if (key === "performanceMonitorPowerMode") {
        return [monitorPowerMode, setMonitorPowerMode, false] as never;
      }
      return [null, setCompactExpanded, true] as never;
    });

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
      order: [
        "processTable",
        "usageGraphs",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["usageGraphs", "processTable", "power"],
    });
  });

  it("allows hiding the final visible panel because instruments stay mounted", async () => {
    customLayout = {
      order: [
        "usageGraphs",
        "processTable",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["usageGraphs"],
    };
    const { result } = renderHook(() => usePerformanceLayout());

    let changed = false;
    await act(async () => {
      changed = await result.current.togglePanel("usageGraphs");
    });

    expect(changed).toBe(true);
    expect(setCustomLayout).toHaveBeenCalledWith({
      order: [
        "usageGraphs",
        "processTable",
        "perCore",
        "motherboardSensors",
        "power",
      ],
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
      order: [
        "usageGraphs",
        "processTable",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["processTable", "power"],
    });

    rerender();

    await act(async () => {
      resolveFirstWrite?.();
      await Promise.all([firstMutation, secondMutation]);
    });

    expect(setCustomLayout).toHaveBeenCalledTimes(2);
    expect(setCustomLayout).toHaveBeenLastCalledWith({
      order: [
        "usageGraphs",
        "processTable",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["power"],
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
