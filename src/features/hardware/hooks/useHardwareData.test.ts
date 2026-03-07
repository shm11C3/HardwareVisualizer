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

import { useHardwareUpdater } from "@/features/hardware/hooks/useHardwareData";
import {
  gpuFanSpeedAtom,
  gpuTempAtom,
} from "@/features/hardware/store/chart";

// Commands (mock targets)
import { commands } from "@/rspc/bindings";

// ------
// Mock setup for each command
// ------
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getNvidiaGpuCooler: vi.fn(),
    getGpuTemperature: vi.fn(),
  },
}));

// ------
// Test body
// ------

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
});
