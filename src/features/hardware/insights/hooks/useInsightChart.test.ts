import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useInsightChart } from "@/features/hardware/insights/hooks/useInsightChart";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { commands } from "@/rspc/bindings";

const hoisted = vi.hoisted(() => ({
  dialogMessageMock: vi.fn(),
  getDataArchiveSeriesMock: vi.fn().mockResolvedValue({
    status: "ok",
    data: [],
  }),
  getGpuArchiveSeriesMock: vi.fn().mockResolvedValue({
    status: "ok",
    data: [],
  }),
}));

// A failed read is a panel state, never a native dialog. If a dialog hook is
// ever wired back into this hook, it ends up calling these.
vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: hoisted.dialogMessageMock,
  ask: hoisted.dialogMessageMock,
  confirm: hoisted.dialogMessageMock,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getDataArchiveSeries: hoisted.getDataArchiveSeriesMock,
    getGpuArchiveSeries: hoisted.getGpuArchiveSeriesMock,
  },
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: vi.fn().mockReturnValue({
    settings: { temperatureUnit: "C" },
  }),
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

  it("keeps an empty answer distinct from a failed read", async () => {
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(ok([]));

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

    expect(commands.getDataArchiveSeries).toHaveBeenCalled();
    expect(result.current.hasData).toBe(false);
    expect(result.current.hasError).toBe(false);
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

    const renderWithOffset = async (offset: number) => {
      const rendered = renderHook(() =>
        useInsightChart({
          hardwareType: "cpu",
          dataStats: "avg",
          period: 10,
          offset,
        }),
      );

      await act(async () => {
        await new Promise((resolve) => setTimeout(resolve, 100));
      });

      return rendered;
    };

    const current = await renderWithOffset(0);
    current.unmount();
    const { result } = await renderWithOffset(5);

    expect(result.current.labels.length).toBeGreaterThan(0);

    // The clock is frozen at 00:02:00Z and a 10 minute period uses a 60s step.
    // The current window ends one archive interval before now; offset 5 moves
    // both ends of that window back by exactly five steps.
    expect(commands.getDataArchiveSeries).toHaveBeenCalledTimes(2);
    expect(commands.getDataArchiveSeries).toHaveBeenNthCalledWith(
      1,
      "cpu",
      "avg",
      "2022-12-31T23:51:00.000Z",
      "2023-01-01T00:01:00.000Z",
      60_000,
      "end",
    );
    expect(commands.getDataArchiveSeries).toHaveBeenNthCalledWith(
      2,
      "cpu",
      "avg",
      "2022-12-31T23:46:00.000Z",
      "2022-12-31T23:56:00.000Z",
      60_000,
      "end",
    );
  });

  it("reports a failed archive read as hasError without opening a dialog", async () => {
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
    expect(result.current.chartData).toEqual([]);
    expect(result.current.hasError).toBe(true);
    // The technical detail goes to the console, not to the user.
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        message: "Failed to fetch archived hardware series: decode failed",
      }),
    );
    expect(hoisted.dialogMessageMock).not.toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });

  it("refetches on retry and clears hasError once the read succeeds", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValueOnce(
      err("decode failed"),
    );
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(
      ok([
        { value: 15, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
    );

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
    expect(result.current.hasError).toBe(true);
    expect(commands.getDataArchiveSeries).toHaveBeenCalledTimes(1);

    act(() => {
      result.current.retry();
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(commands.getDataArchiveSeries).toHaveBeenCalledTimes(2);
    expect(result.current.hasError).toBe(false);
    expect(result.current.chartData).toEqual([15]);
    expect(hoisted.dialogMessageMock).not.toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });

  it("does not let a superseded request's failure set hasError", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    vi.setSystemTime(new Date("2023-01-01T00:02:00Z"));
    let rejectFirst: (reason: Error) => void = () => {};
    vi.mocked(commands.getDataArchiveSeries).mockImplementationOnce(
      () =>
        new Promise((_, reject) => {
          rejectFirst = reject;
        }),
    );
    vi.mocked(commands.getDataArchiveSeries).mockResolvedValue(
      ok([
        { value: 15, timestamp: new Date("2023-01-01T00:01:00Z").getTime() },
      ]),
    );

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    // The user presses retry while the first read is still in flight.
    act(() => {
      result.current.retry();
    });
    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });
    expect(result.current.chartData).toEqual([15]);

    await act(async () => {
      rejectFirst(new Error("late failure"));
      await new Promise((resolve) => setTimeout(resolve, 20));
    });

    expect(result.current.hasError).toBe(false);
    expect(result.current.chartData).toEqual([15]);
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

  it("reports an interval refresh failure as hasError and recovers on the next tick", async () => {
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
    expect(hoisted.dialogMessageMock).not.toHaveBeenCalled();
    expect(result.current.hasData).toBe(false);
    expect(result.current.hasError).toBe(true);

    // Automatic refreshes keep retrying on their own; no user action needed.
    await act(async () => {
      vi.advanceTimersByTime(60000);
      await Promise.resolve();
    });

    expect(result.current.hasError).toBe(false);
    expect(result.current.hasData).toBe(true);
    consoleErrorSpy.mockRestore();
  });
});
