import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useInsightChart } from "@/features/hardware/insights/hooks/useInsightChart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { commands } from "@/rspc/bindings";

const hoisted = vi.hoisted(() => ({
  getDataArchiveRecordsMock: vi.fn().mockResolvedValue({
    status: "ok",
    data: [],
  }),
  getGpuArchiveRecordsMock: vi.fn().mockResolvedValue({
    status: "ok",
    data: [],
  }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getDataArchiveRecords: hoisted.getDataArchiveRecordsMock,
    getGpuArchiveRecords: hoisted.getGpuArchiveRecordsMock,
  },
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: vi.fn().mockReturnValue({
    settings: { temperatureUnit: "C" },
  }),
}));

vi.mock("@/consts", () => ({
  chartConfig: {
    archiveUpdateIntervalMilSec: 60000,
  },
}));

const ok = <T>(data: T) => ({ status: "ok" as const, data });
const err = (error: string) => ({ status: "error" as const, error });

describe("useInsightChart", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should fetch and aggregate data for non-GPU hardware", async () => {
    const mockData = [
      { id: 1, value: 10, timestamp: "2023-01-01T00:00:00Z" },
      { id: 2, value: 20, timestamp: "2023-01-01T00:01:00Z" },
    ];
    vi.mocked(commands.getDataArchiveRecords).mockResolvedValue(ok(mockData));

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels).toHaveLength(11);
    expect(result.current.chartData).toContain(10);
    expect(result.current.chartData).toContain(20);
  });

  it("should fetch and aggregate memory max", async () => {
    const mockData = [
      { id: 1, value: 2000, timestamp: "2023-01-01T00:00:00Z" },
      { id: 2, value: 3000, timestamp: "2023-01-01T00:01:00Z" },
    ];
    vi.mocked(commands.getDataArchiveRecords).mockResolvedValue(ok(mockData));

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "memory",
        dataStats: "max",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.chartData).toContain(3000);
  });

  it("should fetch and aggregate data for GPU hardware", async () => {
    const mockData = [
      { id: 1, value: 30, timestamp: "2023-01-01T00:00:00Z" },
      { id: 2, value: 40, timestamp: "2023-01-01T00:01:00Z" },
    ];
    vi.mocked(commands.getGpuArchiveRecords).mockResolvedValue(ok(mockData));

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "gpu",
        dataStats: "max",
        dataType: "usage",
        period: 10,
        offset: 0,
        gpuName: "NVIDIA",
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels).toHaveLength(11);
    expect(result.current.chartData).toContain(40); // Max of mockData
  });

  it("should fetch and aggregate GPU temperature with min", async () => {
    const mockData = [
      { id: 1, value: 60, timestamp: "2023-01-01T00:00:00Z" },
      { id: 2, value: 50, timestamp: "2023-01-01T00:01:00Z" },
    ];
    vi.mocked(commands.getGpuArchiveRecords).mockResolvedValue(ok(mockData));

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "gpu",
        dataStats: "min",
        dataType: "temp",
        period: 10,
        offset: 0,
        gpuName: "Intel",
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.chartData).toContain(50);
  });

  const mockedTime = new Date("2023-01-01T00:02:00Z");
  vi.setSystemTime(mockedTime);

  it("should handle empty data gracefully", async () => {
    vi.mocked(commands.getDataArchiveRecords).mockResolvedValue(ok([]));

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "memory",
        dataStats: "min",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels).toHaveLength(11);
    expect(result.current.chartData).toEqual(Array(11).fill(null));
  });

  it("should calculate labels correctly for long periods", async () => {
    vi.mocked(commands.getDataArchiveRecords).mockResolvedValue(ok([]));

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 1440,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels).toBeDefined();
    expect(result.current.labels[0]).toMatch(/\d{4}/); // Year should be included
  });

  it("should shift time correctly when offset is applied", async () => {
    const mockData = [{ id: 1, value: 15, timestamp: "2023-01-01T00:00:00Z" }];
    vi.mocked(commands.getDataArchiveRecords).mockResolvedValue(ok(mockData));

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 5,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels.length).toBeGreaterThan(0);
  });

  it("should clear chart data when the archive command returns an error", async () => {
    vi.mocked(commands.getDataArchiveRecords).mockResolvedValue(
      ok([{ id: 1, value: 15, timestamp: "2023-01-01T00:01:00Z" }]),
    );
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));

    const { result, rerender } = renderHook(
      ({ offset }) =>
        useInsightChart({
          hardwareType: "cpu",
          dataStats: "avg",
          period: 10,
          offset,
        }),
      { initialProps: { offset: 0 } },
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.chartData).toContain(15);

    vi.mocked(commands.getDataArchiveRecords).mockResolvedValueOnce(
      err("decode failed"),
    );
    rerender({ offset: 1 });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 300));
    });

    expect(result.current.hasData).toBe(false);
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "Failed to fetch archived hardware records:",
      "decode failed",
    );
    consoleErrorSpy.mockRestore();
  });
});

describe("useInsightChart – formatValue branches", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useSettingsAtom).mockReturnValue({
      settings: { temperatureUnit: "C" },
    } as ReturnType<typeof useSettingsAtom>);
  });

  it("should propagate null values from sqlite as null chart data", async () => {
    vi.mocked(commands.getDataArchiveRecords).mockResolvedValue(
      ok([{ id: 1, value: null, timestamp: "2023-01-01T00:01:00Z" }]),
    );
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // null values must remain null in chartData (not coerced to a number)
    expect(result.current.chartData.some((v) => v === null)).toBe(true);
    expect(result.current.hasData).toBe(false);
  });

  it("should convert temperature from Celsius to Fahrenheit when unit is F", async () => {
    vi.mocked(useSettingsAtom).mockReturnValue({
      settings: { temperatureUnit: "F" },
    } as ReturnType<typeof useSettingsAtom>);
    vi.mocked(commands.getGpuArchiveRecords).mockResolvedValue(
      ok([{ id: 1, value: 100, timestamp: "2023-01-01T00:01:00Z" }]),
    );
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "gpu",
        dataStats: "avg",
        dataType: "temp",
        period: 10,
        offset: 0,
        gpuName: "NVIDIA",
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // 100°C → 212°F
    expect(result.current.chartData).toContain(212);
  });

  it("should convert dedicatedMemory values from KB to GB", async () => {
    vi.mocked(commands.getGpuArchiveRecords).mockResolvedValue(
      ok([
        { id: 1, value: 1048576, timestamp: "2023-01-01T00:01:00Z" }, // 1 GiB in KiB
      ]),
    );
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "gpu",
        dataStats: "avg",
        dataType: "dedicatedMemory",
        period: 10,
        offset: 0,
        gpuName: "NVIDIA",
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // 1 048 576 KiB / 1 024 / 1 024 = 1.0 GB
    expect(result.current.chartData).toContain(1.0);
  });
});

describe("useInsightChart – auto-refresh interval", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));
    vi.mocked(useSettingsAtom).mockReturnValue({
      settings: { temperatureUnit: "C" },
    } as ReturnType<typeof useSettingsAtom>);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("should poll getData via interval when offset is 0", async () => {
    const getDataArchiveRecordsMock = vi.mocked(commands.getDataArchiveRecords);
    getDataArchiveRecordsMock.mockResolvedValue(
      ok([{ id: 1, value: 50, timestamp: "2023-01-01T00:01:00Z" }]),
    );

    renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    // Fire the initial debounced fetch (setTimeout 0)
    await act(async () => {
      vi.advanceTimersByTime(0);
      await Promise.resolve();
    });

    const callsAfterMount = getDataArchiveRecordsMock.mock.calls.length;

    // Advance past archiveUpdateIntervalMilSec (60 000 ms) to trigger interval
    await act(async () => {
      vi.advanceTimersByTime(60000);
      await Promise.resolve();
    });

    expect(getDataArchiveRecordsMock.mock.calls.length).toBeGreaterThan(
      callsAfterMount,
    );
  });

  it("should not start auto-refresh when offset is non-zero", async () => {
    const getDataArchiveRecordsMock = vi.mocked(commands.getDataArchiveRecords);
    getDataArchiveRecordsMock.mockResolvedValue(ok([]));

    renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 5,
      }),
    );

    // Fire initial fetch
    await act(async () => {
      vi.advanceTimersByTime(0);
      await Promise.resolve();
    });

    const callsAfterMount = getDataArchiveRecordsMock.mock.calls.length;

    // Advance well past the interval – should NOT trigger additional fetches
    await act(async () => {
      vi.advanceTimersByTime(120000);
      await Promise.resolve();
    });

    expect(getDataArchiveRecordsMock.mock.calls.length).toBe(callsAfterMount);
  });

  it("cleanup: cancels pending debounce timeout on unmount", () => {
    const clearTimeoutSpy = vi.spyOn(window, "clearTimeout");

    const { unmount } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    // The debounce setTimeout(0) is still pending (timers not advanced).
    // Unmounting should trigger cleanup and cancel it.
    unmount();

    expect(clearTimeoutSpy).toHaveBeenCalled();
    clearTimeoutSpy.mockRestore();
  });

  it("handles getData rejection in interval and logs error", async () => {
    const getDataArchiveRecordsMock = vi.mocked(commands.getDataArchiveRecords);
    getDataArchiveRecordsMock.mockResolvedValue(ok([]));

    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    // Fire initial debounce
    await act(async () => {
      vi.advanceTimersByTime(0);
      await Promise.resolve();
    });

    // Make the command reject on the next call (inside the interval tick)
    getDataArchiveRecordsMock.mockRejectedValueOnce(new Error("DB error"));

    await act(async () => {
      vi.advanceTimersByTime(60000);
      await Promise.resolve();
    });

    expect(consoleErrorSpy).toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });
});
