import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useInsightChart } from "@/features/hardware/insights/hooks/useInsightChart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { commands } from "@/rspc/bindings";

const hoisted = vi.hoisted(() => ({
  errorMock: vi.fn(),
  getDataArchiveSeriesMock: vi.fn().mockResolvedValue({
    status: "ok",
    data: [],
  }),
  getGpuArchiveSeriesMock: vi.fn().mockResolvedValue({
    status: "ok",
    data: [],
  }),
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: hoisted.errorMock }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getDataArchiveSeries: hoisted.getDataArchiveSeriesMock,
    getGpuArchiveSeries: hoisted.getGpuArchiveSeriesMock,
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
    vi.mocked(useSettingsAtom).mockReturnValue({
      settings: { temperatureUnit: "C" },
    } as ReturnType<typeof useSettingsAtom>);
  });

  it("should render the Core-owned series without frontend aggregation", async () => {
    const mockData = [
      { value: 10, timestamp: new Date("2023-01-01T00:00:00Z").getTime() },
      { value: 20, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
    ];
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(ok(mockData));

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

    expect(result.current.labels).toHaveLength(2);
    expect(result.current.chartData).toEqual([10, 20]);
    expect(commands.getDataArchiveSeries).toHaveBeenCalledWith(
      "cpu",
      "avg",
      expect.any(String),
      expect.any(String),
      60_000,
      "end",
    );
  });

  it("should render memory max series values", async () => {
    const mockData = [
      { value: 2000, timestamp: new Date("2023-01-01T00:00:00Z").getTime() },
      { value: 3000, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
    ];
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(ok(mockData));

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

  it("should fetch CPU temperature from the CPU temperature archive column", async () => {
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(
      ok([
        { value: 52, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
    );
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpuTemperature",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(commands.getDataArchiveSeries).toHaveBeenCalledWith(
      "cpuTemperature",
      "avg",
      expect.any(String),
      expect.any(String),
      60_000,
      "end",
    );
    expect(result.current.chartData).toContain(52);
  });

  it("should fetch package power without temperature conversion", async () => {
    vi.mocked(useSettingsAtom).mockReturnValue({
      settings: { temperatureUnit: "F" },
    } as ReturnType<typeof useSettingsAtom>);
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(
      ok([
        { value: 18.4, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
    );
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "packagePower",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(commands.getDataArchiveSeries).toHaveBeenCalledWith(
      "packagePower",
      "avg",
      expect.any(String),
      expect.any(String),
      60_000,
      "end",
    );
    expect(result.current.chartData).toContain(18.4);
  });

  it("should render a GPU archive series", async () => {
    const mockData = [
      { value: 30, timestamp: new Date("2023-01-01T00:00:00Z").getTime() },
      { value: 40, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
    ];
    vi.mocked(commands.getGpuArchiveSeries).mockResolvedValue(ok(mockData));

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

    expect(result.current.labels).toHaveLength(2);
    expect(result.current.chartData).toContain(40); // Max of mockData
  });

  it("should render a minimum GPU temperature series", async () => {
    const mockData = [
      { value: 60, timestamp: new Date("2023-01-01T00:00:00Z").getTime() },
      { value: 50, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
    ];
    vi.mocked(commands.getGpuArchiveSeries).mockResolvedValue(ok(mockData));

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
    const nullSeries = Array.from({ length: 11 }, (_, index) => ({
      timestamp: new Date("2023-01-01T00:00:00Z").getTime() + index * 60_000,
      value: null,
    }));
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(ok(nullSeries));

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
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(
      ok([
        {
          timestamp: new Date("2023-01-01T00:00:00Z").getTime(),
          value: null,
        },
      ]),
    );

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
    const mockData = [
      { value: 15, timestamp: new Date("2023-01-01T00:00:00Z").getTime() },
    ];
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(ok(mockData));

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
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(
      ok([
        { value: 15, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
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

    vi.mocked(commands.getDataArchiveSeries).mockResolvedValueOnce(
      err("decode failed"),
    );
    rerender({ offset: 1 });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 300));
    });

    expect(result.current.hasData).toBe(false);
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        message: "Failed to fetch archived hardware series: decode failed",
      }),
    );
    expect(hoisted.errorMock).toHaveBeenCalledWith(
      "Error: Failed to fetch archived hardware series: decode failed",
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
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(
      ok([
        { value: null, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
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
    vi.mocked(commands.getGpuArchiveSeries).mockResolvedValue(
      ok([
        { value: 100, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
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

  it("should convert CPU temperature from Celsius to Fahrenheit", async () => {
    vi.mocked(useSettingsAtom).mockReturnValue({
      settings: { temperatureUnit: "F" },
    } as ReturnType<typeof useSettingsAtom>);
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(
      ok([
        { value: 50, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
    );
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpuTemperature",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.chartData).toContain(122);
  });

  it("should convert dedicatedMemory values from KB to GB", async () => {
    vi.mocked(commands.getGpuArchiveSeries).mockResolvedValue(
      ok([
        {
          value: 1048576,
          timestamp: new Date("2023-01-01T00:01:00Z").getTime(),
        }, // 1 GiB in KiB
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
    const getDataArchiveSeriesMock = vi.mocked(commands.getDataArchiveSeries);
    getDataArchiveSeriesMock.mockResolvedValue(
      ok([
        { value: 50, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
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

    const callsAfterMount = getDataArchiveSeriesMock.mock.calls.length;

    // Advance past archiveUpdateIntervalMilSec (60 000 ms) to trigger interval
    await act(async () => {
      vi.advanceTimersByTime(60000);
      await Promise.resolve();
    });

    expect(getDataArchiveSeriesMock.mock.calls.length).toBeGreaterThan(
      callsAfterMount,
    );
  });

  it("should not start auto-refresh when offset is non-zero", async () => {
    const getDataArchiveSeriesMock = vi.mocked(commands.getDataArchiveSeries);
    getDataArchiveSeriesMock.mockResolvedValue(ok([]));

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

    const callsAfterMount = getDataArchiveSeriesMock.mock.calls.length;

    // Advance well past the interval – should NOT trigger additional fetches
    await act(async () => {
      vi.advanceTimersByTime(120000);
      await Promise.resolve();
    });

    expect(getDataArchiveSeriesMock.mock.calls.length).toBe(callsAfterMount);
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

  it("handles getData rejection in interval and clears stale chart data", async () => {
    const getDataArchiveSeriesMock = vi.mocked(commands.getDataArchiveSeries);
    getDataArchiveSeriesMock.mockResolvedValue(
      ok([
        { value: 50, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
    );

    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(() =>
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

    expect(result.current.hasData).toBe(true);

    // Make the command reject on the next call (inside the interval tick)
    getDataArchiveSeriesMock.mockRejectedValueOnce(new Error("DB error"));

    await act(async () => {
      vi.advanceTimersByTime(60000);
      await Promise.resolve();
    });

    expect(consoleErrorSpy).toHaveBeenCalled();
    expect(hoisted.errorMock).toHaveBeenCalledWith("Error: DB error");
    expect(result.current.hasData).toBe(false);
    consoleErrorSpy.mockRestore();
  });
});
