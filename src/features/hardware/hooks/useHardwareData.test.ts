// src/features/hardware/hooks/useHardwareData.test.ts
import { act, renderHook, waitFor } from "@testing-library/react";
import { Provider, useAtom } from "jotai";
import {
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  type Mock,
  vi,
} from "vitest";

import { chartConfig } from "@/features/hardware/consts/chart";
// Hook groups to test
import {
  useHardwareUpdater,
  useUsageUpdater,
} from "@/features/hardware/hooks/useHardwareData";
import {
  cpuUsageHistoryAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
  gpuUsageSourceAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/chart";

// Commands (mock targets)
import { commands } from "@/rspc/bindings";

// ------
// Mock setup for each command
// ------
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getCpuUsage: vi.fn(),
    getMemoryUsage: vi.fn(),
    getGpuUsage: vi.fn(),
    getProcessorsUsage: vi.fn(),
    getNvidiaGpuCooler: vi.fn(),
    getGpuTemperature: vi.fn(),
  },
}));

// ------
// Test body
// ------

describe("useUsageUpdater", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("CPU usage history is updated", async () => {
    // simulate: commands.getCpuUsage returns number 50
    (commands.getCpuUsage as Mock).mockResolvedValue(50);

    const { result } = renderHook(
      () => {
        // Call CPU usage history update hook and get atom value simultaneously
        useUsageUpdater("cpu");
        const [history] = useAtom(cpuUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    // useUsageUpdater calls fetchAndUpdate immediately on mount, so
    // verify atom update state with waitFor
    await waitFor(() => {
      const history = result.current;
      expect(history.length).toEqual(chartConfig.historyLengthSec);
      expect(history[history.length - 1]).toEqual(50);
    });
  });

  it("GPU usage history is updated", async () => {
    // simulate: commands.getGpuUsage returns { data: 75 }
    (commands.getGpuUsage as Mock).mockResolvedValue({
      status: "ok",
      data: 75,
    });

    const { result } = renderHook(
      () => {
        useUsageUpdater("gpu");
        const [history] = useAtom(graphicUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      const history = result.current;
      expect(history.length).toEqual(chartConfig.historyLengthSec);
      expect(history[history.length - 1]).toEqual(75);
    });
  });

  it("RAM usage history is updated", async () => {
    // simulate: commands.getMemoryUsage returns number 50
    (commands.getMemoryUsage as Mock).mockResolvedValue(50);

    const { result } = renderHook(
      () => {
        // Call memory usage history update hook and get atom value simultaneously
        useUsageUpdater("memory");
        const [history] = useAtom(memoryUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    // useUsageUpdater calls fetchAndUpdate immediately on mount, so
    // verify atom update state with waitFor
    await waitFor(() => {
      const history = result.current;
      expect(history.length).toEqual(chartConfig.historyLengthSec);
      expect(history[history.length - 1]).toEqual(50);
    });
  });
});

describe("useHardwareUpdater", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("gpuFanSpeedAtom is updated when 'gpu', 'fan'", async () => {
    (commands.getNvidiaGpuCooler as Mock).mockResolvedValue({
      status: "ok",
      data: [{ name: "test1", value: 100 }],
    });

    const { result } = renderHook(
      () => {
        useHardwareUpdater("gpu", "fan");
        const [data] = useAtom(gpuFanSpeedAtom);
        return data;
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      expect(result.current).toEqual([{ name: "test1", value: 100 }]);
    });
  });

  it("gpuTempAtom is updated when 'gpu', 'temp'", async () => {
    (commands.getGpuTemperature as Mock).mockResolvedValue({
      status: "ok",
      data: [{ name: "test2", value: 70 }],
    });

    const { result } = renderHook(
      () => {
        useHardwareUpdater("gpu", "temp");
        const [data] = useAtom(gpuTempAtom);
        return data;
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      expect(result.current).toEqual([{ name: "test2", value: 70 }]);
    });
  });

  it("cpu temp: does not update atom (not implemented)", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(
      () => {
        useHardwareUpdater("cpu", "temp");
        const [data] = useAtom(gpuTempAtom);
        return data;
      },
      { wrapper: Provider },
    );

    await act(async () => {
      await Promise.resolve();
    });

    // atom should remain empty since cpu temp is not implemented
    expect(result.current).toEqual([]);
    expect(consoleErrorSpy).toHaveBeenCalledWith("Not implemented");
    consoleErrorSpy.mockRestore();
  });

  it("cpu fan: does not update atom (not implemented)", async () => {
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(
      () => {
        useHardwareUpdater("cpu", "fan");
        const [data] = useAtom(gpuFanSpeedAtom);
        return data;
      },
      { wrapper: Provider },
    );

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current).toEqual([]);
    expect(consoleErrorSpy).toHaveBeenCalledWith("Not implemented");
    consoleErrorSpy.mockRestore();
  });
});

describe("useUsageUpdater processors", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("processors usage history is updated with array data", async () => {
    (commands.getProcessorsUsage as Mock).mockResolvedValue([
      [30, 40],
      [50, 60],
    ]);

    const { result } = renderHook(
      () => {
        useUsageUpdater("processors");
        const [history] = useAtom(processorsUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      const history = result.current;
      expect(history.length).toBeGreaterThan(0);
      expect(Array.isArray(history[history.length - 1])).toBe(true);
    });
  });
});

describe("useUsageUpdater – error and GpuUsageResult branches", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("early-returns without updating atom when result is an error", async () => {
    // Covers line 69: isResult(result) && isError(result) → return
    (commands.getGpuUsage as Mock).mockResolvedValue({
      status: "error",
      error: "gpu not found",
    });

    const { result } = renderHook(
      () => {
        useUsageUpdater("gpu");
        const [history] = useAtom(graphicUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    await act(async () => {
      await Promise.resolve();
    });

    // Atom should stay empty because early return prevented any update
    expect(result.current).toEqual([]);
  });

  it("extracts usage and source from GpuUsageResult object", async () => {
    // Covers lines 84-86: rawData is { usage, source } object
    (commands.getGpuUsage as Mock).mockResolvedValue({
      status: "ok",
      data: { usage: 60, source: "nvidia_smi" },
    });

    const { result } = renderHook(
      () => {
        useUsageUpdater("gpu");
        const [history] = useAtom(graphicUsageHistoryAtom);
        const [source] = useAtom(gpuUsageSourceAtom);
        return { history, source };
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      const { history, source } = result.current;
      expect(history[history.length - 1]).toEqual(60);
      expect(source).toEqual("nvidia_smi");
    });
  });
});

describe("useHardwareUpdater – interval re-fetch", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("re-fetches gpu fan data after interval tick", async () => {
    // Covers line 177: the setInterval callback inside useHardwareUpdater
    (commands.getNvidiaGpuCooler as Mock).mockResolvedValue({
      status: "ok",
      data: [{ name: "fan1", value: 1200 }],
    });

    renderHook(
      () => {
        useHardwareUpdater("gpu", "fan");
        return useAtom(gpuFanSpeedAtom);
      },
      { wrapper: Provider },
    );

    // Initial fetch
    await act(async () => {
      await Promise.resolve();
    });

    expect(commands.getNvidiaGpuCooler).toHaveBeenCalledTimes(1);

    // Advance by interval (10 s) to trigger re-fetch
    await act(async () => {
      vi.advanceTimersByTime(10000);
      await Promise.resolve();
    });

    expect(commands.getNvidiaGpuCooler).toHaveBeenCalledTimes(2);
  });

  it("cleanup: cancels interval on unmount (useHardwareUpdater)", async () => {
    // Covers the () => clearInterval(intervalId) cleanup in useHardwareUpdater
    (commands.getNvidiaGpuCooler as Mock).mockResolvedValue({
      status: "ok",
      data: [{ name: "fan1", value: 1200 }],
    });

    const clearIntervalSpy = vi.spyOn(window, "clearInterval");

    const { unmount } = renderHook(() => useHardwareUpdater("gpu", "fan"), {
      wrapper: Provider,
    });

    await act(async () => {
      await Promise.resolve();
    });

    unmount();

    expect(clearIntervalSpy).toHaveBeenCalled();
    clearIntervalSpy.mockRestore();
  });
});

describe("useUsageUpdater – cleanup", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("cleanup: cancels interval on unmount (useUsageUpdater)", async () => {
    // Covers the () => clearInterval(intervalId) cleanup in useUsageUpdater
    (commands.getCpuUsage as Mock).mockResolvedValue(50);

    const clearIntervalSpy = vi.spyOn(window, "clearInterval");

    const { unmount } = renderHook(() => useUsageUpdater("cpu"), {
      wrapper: Provider,
    });

    await act(async () => {
      await Promise.resolve();
    });

    unmount();

    expect(clearIntervalSpy).toHaveBeenCalled();
    clearIntervalSpy.mockRestore();
  });
});
