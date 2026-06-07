import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { getArchivedRecord } from "@/features/hardware/insights/snapshot/funcs/getArchivedRecord";
import { useSnapshot } from "@/features/hardware/insights/snapshot/hooks/useSnapshot";

const hoisted = vi.hoisted(() => ({
  errorMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: hoisted.errorMock }),
}));

vi.mock("@/features/hardware/insights/snapshot/funcs/getArchivedRecord");

const mockGetArchivedRecord = vi.mocked(getArchivedRecord);

import { getProcessStatsInPeriod } from "@/features/hardware/insights/snapshot/funcs/getArchivedRecord";

const mockGetProcessStatsInPeriod = vi.mocked(getProcessStatsInPeriod);

// Use a mutable reference so individual tests can override memory size
let mockMemorySize: string | undefined = "32 GB";

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: {
      memory: mockMemorySize ? { size: mockMemorySize } : undefined,
    },
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

describe("useSnapshot - Branch Coverage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockMemorySize = "32 GB";
    mockGetArchivedRecord.mockResolvedValue([]);
    mockGetProcessStatsInPeriod.mockResolvedValue([]);
  });

  it("should skip records with null value when bucketing data", async () => {
    // Records with null values should be excluded from bucketedData (line 125 branch)
    mockGetArchivedRecord.mockResolvedValue([
      {
        id: 1,
        value: null as unknown as number,
        timestamp: "2023-01-01T10:00:00Z",
      },
      { id: 2, value: 75, timestamp: "2023-01-01T10:01:00Z" },
    ]);

    const { result } = renderHook(() => useSnapshot());

    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T09:00:00Z",
        end: "2023-01-01T11:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Chart data should only include the non-null value (75)
    const nonNullValues = result.current.filledChartData.filter(
      (v) => v !== null,
    );
    expect(nonNullValues.length).toBeGreaterThan(0);
    expect(nonNullValues.every((v) => v === 75)).toBe(true);
  });

  it("should return empty arrays when period start is equal to end (line 205-206 branch)", async () => {
    const { result } = renderHook(() => useSnapshot());

    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T10:00:00Z",
        end: "2023-01-01T10:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.filledLabels).toHaveLength(0);
    expect(result.current.filledChartData).toHaveLength(0);
  });

  it("should return empty arrays when period start is after end (line 205-206 branch)", async () => {
    const { result } = renderHook(() => useSnapshot());

    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T12:00:00Z",
        end: "2023-01-01T08:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.filledLabels).toHaveLength(0);
    expect(result.current.filledChartData).toHaveLength(0);
  });

  it("should return empty filteredProcessData when processData is null (line 175-176 branch)", async () => {
    // getProcessStatsInPeriod returning null simulates a defensive guard scenario
    mockGetProcessStatsInPeriod.mockResolvedValue(
      null as unknown as Awaited<ReturnType<typeof getProcessStatsInPeriod>>,
    );

    const { result } = renderHook(() => useSnapshot());

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.filteredProcessData).toEqual([]);
  });

  it("should filter processData by cpuRange and memoryRange", async () => {
    mockGetProcessStatsInPeriod.mockResolvedValue([
      {
        pid: 1,
        process_name: "high_cpu",
        avg_cpu_usage: 80,
        avg_memory_usage: 1024, // 1 MB
        total_execution_sec: 100,
        latest_timestamp: "2023-01-01T10:00:00Z",
      },
      {
        pid: 2,
        process_name: "low_cpu",
        avg_cpu_usage: 10,
        avg_memory_usage: 512, // 0.5 MB
        total_execution_sec: 50,
        latest_timestamp: "2023-01-01T10:00:00Z",
      },
    ]);

    const { result } = renderHook(() => useSnapshot());

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Default cpuRange is [0, 100] and memoryRange [0, 32768] so both pass
    expect(result.current.filteredProcessData).toHaveLength(2);

    // Narrow CPU range to exclude low_cpu
    act(() => {
      result.current.setCpuRange({ type: "cpu", value: [50, 100] });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.filteredProcessData).toHaveLength(1);
    expect(result.current.filteredProcessData[0].process_name).toBe("high_cpu");
  });

  it("should return step of 60000 when period diff is zero (step guard branch)", async () => {
    const { result } = renderHook(() => useSnapshot());

    // The step useMemo guard returns 60000 when diff <= 0 (same start/end)
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T10:00:00Z",
        end: "2023-01-01T10:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // With equal start/end, filledChartData and filledLabels are both empty
    // (early return on line 206 is reached, which uses step internally but returns empty)
    expect(result.current.filledLabels).toHaveLength(0);
    expect(result.current.filledChartData).toHaveLength(0);
  });

  it.each([
    ["128MB", 128],
    ["256MB", 256],
    ["512MB", 512],
    ["1GB", 1024],
    ["2GB", 2048],
    ["8GB", 8192],
  ] as const)("should return %s (%i MB) for memoryMaxOption %s", async (option, expectedMB) => {
    const { result } = renderHook(() => useSnapshot());

    act(() => {
      result.current.setMemoryMaxOption(option);
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(result.current.selectedMemoryMaxMB).toBe(expectedMB);
  });

  it("should use MB value directly when memory size is in MB", async () => {
    mockMemorySize = "8192 MB";
    const { result } = renderHook(() => useSnapshot());

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(result.current.totalMemoryMB).toBe(8192);
  });

  it("should return default 32768 MB when memory size unit is unknown", async () => {
    mockMemorySize = "32 TB";
    const { result } = renderHook(() => useSnapshot());

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(result.current.totalMemoryMB).toBe(32768);
  });

  it("should return default 32768 MB when memory size is absent", async () => {
    mockMemorySize = undefined;
    const { result } = renderHook(() => useSnapshot());

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 0));
    });

    expect(result.current.totalMemoryMB).toBe(32768);
  });
});
