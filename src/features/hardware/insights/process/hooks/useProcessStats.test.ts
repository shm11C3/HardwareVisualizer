import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { useProcessStats } from "@/features/hardware/insights/process/hooks/useProcessStats";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";

// Hoisted mocks to comply with Vitest hoisting behavior
const hoisted = vi.hoisted(() => ({
  errorMock: vi.fn(),
  getProcessStatsMock: vi.fn(),
  atomState: null as ProcessStat[] | null,
  setProcessStatsAtomMock: vi.fn((v: ProcessStat[]) => {
    hoisted.atomState = v;
  }),
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: hoisted.errorMock }),
}));

// Use a predictable archive interval (60s)
vi.mock("@/features/hardware/consts/chart", () => ({
  chartConfig: { archiveUpdateIntervalMilSec: 60000 },
}));

vi.mock(
  "@/features/hardware/insights/process/funcs/getProcessStatsRecord",
  () => ({
    getProcessStats: hoisted.getProcessStatsMock,
  }),
);

// Mock the atom hook to control and observe state changes
vi.mock(
  "@/features/hardware/insights/process/hooks/useProcessStatsAtom",
  () => ({
    useProcessStatsAtom: () => ({
      processStats: hoisted.atomState,
      setProcessStatsAtom: hoisted.setProcessStatsAtomMock,
    }),
  }),
);

describe("useProcessStats", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // reset timers/date mocks between tests
    vi.useRealTimers();
    hoisted.atomState = null;
  });

  it("fetches stats on mount and updates atom + loading", async () => {
    const now = new Date("2023-01-01T00:10:00Z");
    vi.setSystemTime(now);

    const mockData = [
      {
        pid: 1,
        process_name: "foo",
        avg_cpu_usage: 10,
        avg_memory_usage: 100,
        total_execution_sec: 60,
        latest_timestamp: now.toISOString(),
      },
    ];
    (hoisted.getProcessStatsMock as Mock).mockResolvedValueOnce(mockData);

    const { result } = renderHook(() =>
      useProcessStats({ period: 10 as const, offset: 2 }),
    );

    // Allow effect/promise to resolve
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });

    // endAt should be now - offset * step (2 * 60000 = 120000 ms)
    const expectedEndAt = new Date(now.getTime() - 2 * 60000);
    const [, passedEndAt] = (hoisted.getProcessStatsMock as Mock).mock.calls[0];
    expect((passedEndAt as Date).toISOString()).toBe(
      expectedEndAt.toISOString(),
    );

    expect(hoisted.setProcessStatsAtomMock).toHaveBeenCalledWith(mockData);
    expect(result.current.loading).toBe(false);
    expect(result.current.processStats).toEqual(mockData);
  });

  it("refreshes periodically via interval", async () => {
    vi.useFakeTimers();
    const now = new Date("2023-01-01T00:00:00Z");
    vi.setSystemTime(now);

    const first = [
      {
        pid: 1,
        process_name: "foo",
        avg_cpu_usage: 1,
        avg_memory_usage: 10,
        total_execution_sec: 10,
        latest_timestamp: now.toISOString(),
      },
    ];
    const second = [
      {
        pid: 2,
        process_name: "bar",
        avg_cpu_usage: 2,
        avg_memory_usage: 20,
        total_execution_sec: 20,
        latest_timestamp: now.toISOString(),
      },
    ];

    (hoisted.getProcessStatsMock as Mock)
      .mockResolvedValueOnce(first)
      .mockResolvedValueOnce(second);

    const { result } = renderHook(() =>
      useProcessStats({ period: 10 as const, offset: 0 }),
    );

    // Resolve initial fetch
    await act(async () => {
      await Promise.resolve();
    });
    expect(result.current.processStats).toEqual(first);

    // Advance 60 seconds to trigger interval
    await act(async () => {
      vi.advanceTimersByTime(60000);
      await Promise.resolve();
    });

    expect(result.current.processStats).toEqual(second);

    vi.useRealTimers();
  });

  it("handles errors by showing dialog and stopping loading", async () => {
    const now = new Date("2023-01-01T00:00:00Z");
    vi.setSystemTime(now);

    (hoisted.getProcessStatsMock as Mock).mockRejectedValueOnce(
      new Error("boom"),
    );

    const { result } = renderHook(() =>
      useProcessStats({ period: 10 as const, offset: 0 }),
    );

    await act(async () => {
      await Promise.resolve();
    });

    expect(hoisted.errorMock).toHaveBeenCalledWith("Error: boom");
    expect(result.current.loading).toBe(false);
    expect(result.current.processStats).toBeNull();
  });

  it("cleans up interval on unmount", async () => {
    vi.useFakeTimers();
    const now = new Date("2023-01-01T00:00:00Z");
    vi.setSystemTime(now);

    (hoisted.getProcessStatsMock as Mock).mockResolvedValue([]);

    const { unmount } = renderHook(() =>
      useProcessStats({ period: 10 as const, offset: 0 }),
    );

    // Resolve initial fetch
    await act(async () => {
      await Promise.resolve();
    });

    const callsAfterFirst = (hoisted.getProcessStatsMock as Mock).mock.calls
      .length;

    unmount();

    // Advance time; no further calls should occur
    await act(async () => {
      vi.advanceTimersByTime(120000);
      await Promise.resolve();
    });

    expect((hoisted.getProcessStatsMock as Mock).mock.calls.length).toBe(
      callsAfterFirst,
    );

    vi.useRealTimers();
  });
});
