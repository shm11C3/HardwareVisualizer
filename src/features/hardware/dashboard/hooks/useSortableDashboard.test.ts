import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriStore } from "@/hooks/useTauriStore";
import { type DashboardItemType, dashBoardItems } from "../types/dashboardItem";

const mockInit = vi.fn();
const mockSetDashboardItemMap = vi.fn();

vi.mock("../../hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    init: mockInit,
    hardwareInfo: {},
    networkInfo: [],
  }),
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi
    .fn()
    .mockImplementation((_key: string, defaultValue: DashboardItemType[]) => [
      defaultValue,
      mockSetDashboardItemMap,
      false,
    ]),
}));

vi.mock("@dnd-kit/sortable", () => ({
  arraySwap: vi.fn((arr: unknown[], a: number, b: number) => {
    const copy = [...arr];
    [copy[a], copy[b]] = [copy[b], copy[a]];
    return copy;
  }),
}));

import { useSortableDashboard } from "./useSortableDashboard";

describe("useSortableDashboard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls init on mount", () => {
    renderHook(() => useSortableDashboard(), { wrapper: Provider });
    expect(mockInit).toHaveBeenCalledTimes(1);
  });

  it("returns dashboardItemMap from store", () => {
    const { result } = renderHook(() => useSortableDashboard(), {
      wrapper: Provider,
    });
    expect(result.current.dashboardItemMap).toEqual([...dashBoardItems]);
  });

  it("handleDragOver: swaps items when active and over differ", () => {
    const { result } = renderHook(() => useSortableDashboard(), {
      wrapper: Provider,
    });

    act(() => {
      result.current.handleDragOver({
        active: { id: "cpu" },
        over: { id: "gpu" },
      } as never);
    });

    expect(mockSetDashboardItemMap).toHaveBeenCalledTimes(1);
  });

  it("handleDragOver: does nothing when over is null", () => {
    const { result } = renderHook(() => useSortableDashboard(), {
      wrapper: Provider,
    });

    act(() => {
      result.current.handleDragOver({
        active: { id: "cpu" },
        over: null,
      } as never);
    });

    expect(mockSetDashboardItemMap).not.toHaveBeenCalled();
  });

  it("handleDragOver: does nothing when active.id === over.id", () => {
    const { result } = renderHook(() => useSortableDashboard(), {
      wrapper: Provider,
    });

    act(() => {
      result.current.handleDragOver({
        active: { id: "cpu" },
        over: { id: "cpu" },
      } as never);
    });

    expect(mockSetDashboardItemMap).not.toHaveBeenCalled();
  });

  it("handleDragOver: does nothing when dashboardItemMap is null (store not yet loaded)", () => {
    // Simulate the pending state where the Tauri store hasn't loaded yet
    vi.mocked(useTauriStore).mockReturnValueOnce([
      null,
      mockSetDashboardItemMap,
      true,
    ]);

    const { result } = renderHook(() => useSortableDashboard(), {
      wrapper: Provider,
    });

    act(() => {
      result.current.handleDragOver({
        active: { id: "cpu" },
        over: { id: "gpu" },
      } as never);
    });

    expect(mockSetDashboardItemMap).not.toHaveBeenCalled();
  });
});
