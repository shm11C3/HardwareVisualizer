// src/features/hardware/hooks/useHardwareData.test.ts
import { act, renderHook, waitFor } from "@testing-library/react";
import { Provider, useAtom } from "jotai";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

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
