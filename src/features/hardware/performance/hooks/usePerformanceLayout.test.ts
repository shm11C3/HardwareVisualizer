import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
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
    vi.mocked(useTauriStore).mockImplementation((key) =>
      key === "performanceLayoutPreset"
        ? ([preset, setPreset, false] as never)
        : ([customLayout, setCustomLayout, false] as never),
    );
  });

  it("persists preset selection in UI-local store", async () => {
    const { result } = renderHook(() => usePerformanceLayout());

    await act(async () => result.current.setPreset("monitor"));

    expect(setPreset).toHaveBeenCalledWith("monitor");
  });

  it("reorders Custom panels without changing visibility", () => {
    const { result } = renderHook(() => usePerformanceLayout());

    act(() => {
      result.current.handlePanelDragEnd({
        active: { id: "processTable" },
        over: { id: "currentValues" },
      } as never);
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
});
