import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useProcessStatsAtom } from "@/features/hardware/insights/process/hooks/useProcessStatsAtom";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";

describe("useProcessStatsAtom", () => {
  it("initial state is null", () => {
    const { result } = renderHook(() => useProcessStatsAtom());
    expect(result.current.processStats).toBeNull();
  });

  it("setProcessStatsAtom updates processStats", () => {
    const { result } = renderHook(() => useProcessStatsAtom());

    const mockStats: ProcessStat[] = [
      {
        pid: 1,
        process_name: "chrome",
        avg_cpu_usage: 5.0,
        avg_memory_usage: 200,
        total_execution_sec: 3600,
        latest_timestamp: "2023-01-01T00:00:00Z",
      },
    ];

    act(() => {
      result.current.setProcessStatsAtom(mockStats);
    });

    expect(result.current.processStats).toEqual(mockStats);
  });

  it("setProcessStatsAtom updates to empty array", () => {
    const { result } = renderHook(() => useProcessStatsAtom());

    act(() => {
      result.current.setProcessStatsAtom([]);
    });

    expect(result.current.processStats).toEqual([]);
  });
});
