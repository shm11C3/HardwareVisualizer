import { act, renderHook } from "@testing-library/react";
import { Provider, useAtom } from "jotai";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { chartConfig } from "@/features/hardware/consts/chart";
import { useHardwareEventListener } from "@/features/hardware/hooks/useHardwareEventListener";
import {
  cpuUsageHistoryAtom,
  gpuDedicatedMemoryKbAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
  gpuUsageSourceAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import type {
  GpuMonitorData,
  HardwareMonitorUpdate,
} from "@/rspc/bindings";

// ── Mock ──

type ListenCallback = (event: { payload: HardwareMonitorUpdate }) => void;

const mockUnlisten = vi.fn();
let capturedCallback: ListenCallback | null = null;

vi.mock("@/rspc/bindings", () => ({
  events: {
    hardwareMonitorUpdate: {
      listen: vi.fn((cb: ListenCallback) => {
        capturedCallback = cb;
        return Promise.resolve(mockUnlisten);
      }),
    },
  },
}));

// ── Helpers ──

const emit = (payload: HardwareMonitorUpdate) => {
  if (!capturedCallback) throw new Error("listener not registered");
  capturedCallback({ payload });
};

const makeGpu = (
  overrides: Partial<GpuMonitorData> = {},
): GpuMonitorData => ({
  gpuId: "nvapi:0",
  gpuName: "TestGPU",
  gpuUsage: 70,
  gpuTemperature: 65,
  gpuSource: "IOKit",
  gpuDedicatedMemoryUsageKb: null,
  gpuCoolerLevel: null,
  ...overrides,
});

const makePayload = (
  overrides: Partial<Omit<HardwareMonitorUpdate, "gpus">> & {
    gpus?: GpuMonitorData[];
  } = {},
): HardwareMonitorUpdate => {
  const { gpus, ...rest } = overrides;
  return {
    cpuUsage: 50,
    memoryUsage: 60,
    gpus: gpus ?? [makeGpu()],
    processorsUsage: [40, 50],
    ...rest,
  };
};

// ── Tests ──

describe("useHardwareEventListener", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    capturedCallback = null;
  });

  // ── Listener registration / cleanup ──

  it("registers event listener on mount", () => {
    renderHook(() => useHardwareEventListener(), { wrapper: Provider });

    expect(capturedCallback).not.toBeNull();
  });

  it("calls unlisten on unmount", async () => {
    const { unmount } = renderHook(() => useHardwareEventListener(), {
      wrapper: Provider,
    });

    unmount();
    await act(async () => {
      await Promise.resolve();
    });

    expect(mockUnlisten).toHaveBeenCalledTimes(1);
  });

  // ── CPU history ──

  it("appends cpuUsage to cpuUsageHistoryAtom", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [history] = useAtom(cpuUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    act(() => emit(makePayload({ cpuUsage: 42 })));

    const history = result.current;
    expect(history).toHaveLength(chartConfig.historyLengthSec);
    expect(history[history.length - 1]).toBe(42);
  });

  // ── Memory history ──

  it("appends memoryUsage to memoryUsageHistoryAtom", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [history] = useAtom(memoryUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    act(() => emit(makePayload({ memoryUsage: 75 })));

    const history = result.current;
    expect(history).toHaveLength(chartConfig.historyLengthSec);
    expect(history[history.length - 1]).toBe(75);
  });

  // ── Padding ──

  it("pads history with null when shorter than historyLengthSec", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [history] = useAtom(cpuUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    act(() => emit(makePayload({ cpuUsage: 10 })));

    const history = result.current;
    expect(history).toHaveLength(chartConfig.historyLengthSec);
    // First elements are null padding
    expect(history[0]).toBeNull();
    expect(history[history.length - 1]).toBe(10);
  });

  // ── History limit ──

  it("trims history exceeding historyLengthSec", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [history] = useAtom(cpuUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    // Emit historyLengthSec + 5 events
    act(() => {
      for (let i = 0; i < chartConfig.historyLengthSec + 5; i++) {
        emit(makePayload({ cpuUsage: i }));
      }
    });

    const history = result.current;
    expect(history).toHaveLength(chartConfig.historyLengthSec);
    // Oldest values should have been dropped
    expect(history[0]).toBe(5);
    expect(history[history.length - 1]).toBe(chartConfig.historyLengthSec + 4);
  });

  // ── GPU usage ──

  it("appends gpuUsage to graphicUsageHistoryAtom when not null", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [history] = useAtom(graphicUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    act(() => emit(makePayload({ gpus: [makeGpu({ gpuUsage: 85 })] })));

    const history = result.current;
    expect(history[history.length - 1]).toBe(85);
  });

  it("does not update graphicUsageHistoryAtom when gpuUsage is null", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [history] = useAtom(graphicUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    act(() => emit(makePayload({ gpus: [makeGpu({ gpuUsage: null })] })));

    expect(result.current).toEqual([]);
  });

  // ── GPU temperature ──

  it("updates gpuTempAtom as NameValue[] when gpuName and gpuTemperature are present", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [temp] = useAtom(gpuTempAtom);
        return temp;
      },
      { wrapper: Provider },
    );

    act(() =>
      emit(
        makePayload({
          gpus: [makeGpu({ gpuName: "RTX 4090", gpuTemperature: 72 })],
        }),
      ),
    );

    expect(result.current).toEqual([{ name: "RTX 4090", value: 72 }]);
  });

  it("does not update gpuTempAtom when gpuTemperature is null", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [temp] = useAtom(gpuTempAtom);
        return temp;
      },
      { wrapper: Provider },
    );

    act(() =>
      emit(makePayload({ gpus: [makeGpu({ gpuTemperature: null })] })),
    );

    expect(result.current).toEqual([]);
  });

  // ── GPU source ──

  it("updates gpuUsageSourceAtom when gpuSource is not null", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [source] = useAtom(gpuUsageSourceAtom);
        return source;
      },
      { wrapper: Provider },
    );

    act(() =>
      emit(makePayload({ gpus: [makeGpu({ gpuSource: "NVAPI" })] })),
    );

    expect(result.current).toBe("NVAPI");
  });

  it("sets gpuUsageSourceAtom to null when no GPUs are present", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [source] = useAtom(gpuUsageSourceAtom);
        return source;
      },
      { wrapper: Provider },
    );

    act(() => emit(makePayload({ gpus: [] })));

    expect(result.current).toBeNull();
  });

  // ── Processor usage ──

  it("appends processorsUsage to processorsUsageHistoryAtom", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [history] = useAtom(processorsUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    act(() => emit(makePayload({ processorsUsage: [10, 20, 30] })));

    const history = result.current;
    expect(history).toHaveLength(1);
    expect(history[0]).toEqual([10, 20, 30]);
  });

  it("trims processorsUsageHistoryAtom exceeding historyLengthSec", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [history] = useAtom(processorsUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    act(() => {
      for (let i = 0; i < chartConfig.historyLengthSec + 3; i++) {
        emit(makePayload({ processorsUsage: [i, i + 1] }));
      }
    });

    const history = result.current;
    expect(history).toHaveLength(chartConfig.historyLengthSec);
    expect(history[0]).toEqual([3, 4]);
  });

  // ── GPU dedicated memory ──

  it("updates gpuDedicatedMemoryKbAtom when gpuDedicatedMemoryUsageKb is not null", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [mem] = useAtom(gpuDedicatedMemoryKbAtom);
        return mem;
      },
      { wrapper: Provider },
    );

    act(() =>
      emit(
        makePayload({
          gpus: [makeGpu({ gpuDedicatedMemoryUsageKb: 4096 })],
        }),
      ),
    );

    expect(result.current).toBe(4096);
  });

  it("does not update gpuDedicatedMemoryKbAtom when gpuDedicatedMemoryUsageKb is null", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [mem] = useAtom(gpuDedicatedMemoryKbAtom);
        return mem;
      },
      { wrapper: Provider },
    );

    act(() =>
      emit(
        makePayload({
          gpus: [makeGpu({ gpuDedicatedMemoryUsageKb: null })],
        }),
      ),
    );

    expect(result.current).toBeNull();
  });

  // ── GPU fan speed (cooler) ──

  it("updates gpuFanSpeedAtom when gpuCoolerLevel and gpuName are present", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [fan] = useAtom(gpuFanSpeedAtom);
        return fan;
      },
      { wrapper: Provider },
    );

    act(() =>
      emit(
        makePayload({
          gpus: [makeGpu({ gpuName: "RTX 4090", gpuCoolerLevel: 55 })],
        }),
      ),
    );

    expect(result.current).toEqual([{ name: "RTX 4090", value: 55 }]);
  });

  it("does not update gpuFanSpeedAtom when gpuCoolerLevel is null", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [fan] = useAtom(gpuFanSpeedAtom);
        return fan;
      },
      { wrapper: Provider },
    );

    act(() =>
      emit(makePayload({ gpus: [makeGpu({ gpuCoolerLevel: null })] })),
    );

    expect(result.current).toEqual([]);
  });

  // ── Consecutive events ──

  it("accumulates values across multiple events", () => {
    const { result } = renderHook(
      () => {
        useHardwareEventListener();
        const [cpu] = useAtom(cpuUsageHistoryAtom);
        const [mem] = useAtom(memoryUsageHistoryAtom);
        return { cpu, mem };
      },
      { wrapper: Provider },
    );

    act(() => {
      emit(makePayload({ cpuUsage: 10, memoryUsage: 20 }));
      emit(makePayload({ cpuUsage: 30, memoryUsage: 40 }));
      emit(makePayload({ cpuUsage: 50, memoryUsage: 60 }));
    });

    const { cpu, mem } = result.current;
    expect(cpu).toHaveLength(chartConfig.historyLengthSec);
    expect(cpu.slice(-3)).toEqual([10, 30, 50]);
    expect(mem.slice(-3)).toEqual([20, 40, 60]);
  });
});
