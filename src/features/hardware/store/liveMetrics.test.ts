import { createStore } from "jotai";
import { describe, expect, it, vi } from "vitest";
import { chartConfig } from "@/consts/chart";
import { asLiveGpuId } from "@/features/hardware/gpuIdentity";
import {
  liveGpu,
  liveSample,
  paddedHistory,
} from "@/features/hardware/live/liveSamples.testHelpers";
import {
  gpuDedicatedMemoryKbMapAtom,
  gpuFanSpeedMapAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  gpuUsageSourcesAtom,
} from "@/features/hardware/store/gpu";
import {
  clearGpuTemperaturesAtom,
  cpuUsageCurrentAtom,
  cpuUsageSeriesAtom,
  gpuUsageCurrentAtom,
  gpuUsageSeriesAtom,
  latestProcessorUsagesAtom,
  liveMetricsAtom,
  liveScalarAtom,
  liveSeriesAtom,
  memoryUsageCurrentAtom,
  powerCurrentAtom,
  powerDrawSeriesAtom,
  processorUsageSeriesAtom,
  publishLiveSampleAtom,
} from "@/features/hardware/store/liveMetrics";
import {
  cpuUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/liveUsage";
import {
  powerDrawAtom,
  powerDrawHistoryAtom,
} from "@/features/hardware/store/power";

const gpuA = asLiveGpuId("gpu-a");

const publish = (
  store: ReturnType<typeof createStore>,
  overrides: Parameters<typeof liveSample>[0] = {},
  missingSampleCount = 0,
) =>
  store.set(publishLiveSampleAtom, liveSample(overrides), missingSampleCount);

describe("the buffers belong to the store", () => {
  it("starts every store empty, and keeps stores apart", () => {
    const first = createStore();
    const second = createStore();

    publish(first, { cpuUsage: 42 });

    expect(first.get(cpuUsageCurrentAtom)).toBe(42);
    expect(second.get(cpuUsageCurrentAtom)).toBeNull();
    expect(createStore().get(cpuUsageCurrentAtom)).toBeNull();
  });

  it("keeps one set of buffers for the life of a store", () => {
    const store = createStore();
    const buffers = store.get(liveMetricsAtom).buffers;

    publish(store);
    publish(store);

    expect(store.get(liveMetricsAtom).buffers).toBe(buffers);
  });

  it("notifies a subscriber once per published sample, even though the buffers are the same object", () => {
    const store = createStore();
    const onSample = vi.fn();
    store.sub(liveMetricsAtom, onSample);

    publish(store);
    publish(store);

    expect(onSample).toHaveBeenCalledTimes(2);
  });
});

describe("scalars", () => {
  it("are null before a sample and follow the latest one", () => {
    const store = createStore();
    expect(store.get(cpuUsageCurrentAtom)).toBeNull();
    expect(store.get(memoryUsageCurrentAtom)).toBeNull();

    publish(store, { cpuUsage: 10, memoryUsage: 60 });
    publish(store, { cpuUsage: 20, memoryUsage: 61 });

    expect(store.get(cpuUsageCurrentAtom)).toBe(20);
    expect(store.get(memoryUsageCurrentAtom)).toBe(61);
  });

  it("do not notify when a sample leaves the value unchanged", () => {
    const store = createStore();
    publish(store, { cpuUsage: 10, memoryUsage: 60 });
    const onCpu = vi.fn();
    const onMemory = vi.fn();
    store.sub(cpuUsageCurrentAtom, onCpu);
    store.sub(memoryUsageCurrentAtom, onMemory);

    publish(store, { cpuUsage: 10, memoryUsage: 61 });

    expect(onCpu).not.toHaveBeenCalled();
    expect(onMemory).toHaveBeenCalledTimes(1);
  });
});

describe("series", () => {
  it("hand out one atom per processor and per adapter", () => {
    expect(processorUsageSeriesAtom(2)).toBe(processorUsageSeriesAtom(2));
    expect(processorUsageSeriesAtom(2)).not.toBe(processorUsageSeriesAtom(3));
    expect(gpuUsageSeriesAtom(gpuA)).toBe(gpuUsageSeriesAtom(gpuA));
  });

  it("are padded to the window and keep the per-key values apart", () => {
    const store = createStore();
    publish(store, {
      processorsUsage: [1, 2],
      gpus: [liveGpu("gpu-a", { gpuUsage: 7 })],
    });
    publish(store, { processorsUsage: [3, 4] });

    expect(store.get(processorUsageSeriesAtom(0))).toEqual(paddedHistory(1, 3));
    expect(store.get(processorUsageSeriesAtom(1))).toEqual(paddedHistory(2, 4));
    expect(store.get(gpuUsageSeriesAtom(gpuA))).toEqual(paddedHistory(7, null));
  });

  it("read as all gaps for a processor or adapter that has not reported", () => {
    const store = createStore();

    expect(store.get(processorUsageSeriesAtom(5))).toEqual(paddedHistory());
    expect(store.get(gpuUsageSeriesAtom(gpuA))).toHaveLength(
      chartConfig.historyLengthSec,
    );
  });
});

describe("per-adapter and per-domain channels", () => {
  it("read one adapter's current usage, null for a gap or an unknown adapter", () => {
    const store = createStore();
    expect(store.get(gpuUsageCurrentAtom(gpuA))).toBeNull();

    publish(store, { gpus: [liveGpu("gpu-a", { gpuUsage: 7 })] });
    expect(store.get(gpuUsageCurrentAtom(gpuA))).toBe(7);
    expect(store.get(gpuUsageCurrentAtom(asLiveGpuId("gpu-b")))).toBeNull();

    publish(store, { gpus: [liveGpu("gpu-a", { gpuUsage: null })] });
    expect(store.get(gpuUsageCurrentAtom(gpuA))).toBeNull();
  });

  it("read each power domain on its own, in watts", () => {
    const store = createStore();
    expect(store.get(powerCurrentAtom("cpuWatts"))).toBeNull();

    publish(store, { cpuPowerWatts: 12.5, packagePowerWatts: 30 });
    publish(store, { cpuPowerWatts: 13, packagePowerWatts: 30 });

    expect(store.get(powerCurrentAtom("cpuWatts"))).toBe(13);
    expect(store.get(powerCurrentAtom("gpuWatts"))).toBeNull();
    expect(store.get(powerDrawSeriesAtom("cpuWatts"))).toEqual(
      paddedHistory(12.5, 13),
    );
    expect(store.get(powerDrawSeriesAtom("packageWatts"))).toEqual(
      paddedHistory(30, 30),
    );
  });

  it("do not wake a power-domain subscriber when another domain moves", () => {
    const store = createStore();
    publish(store, { cpuPowerWatts: 10, packagePowerWatts: 30 });
    const onCpu = vi.fn();
    store.sub(powerCurrentAtom("cpuWatts"), onCpu);

    publish(store, { cpuPowerWatts: 10, packagePowerWatts: 31 });

    expect(onCpu).not.toHaveBeenCalled();
  });

  it("resolve the same atoms through the channel lookups", () => {
    expect(liveScalarAtom({ kind: "gpu", id: gpuA })).toBe(
      gpuUsageCurrentAtom(gpuA),
    );
    expect(liveScalarAtom({ kind: "power", key: "gpuWatts" })).toBe(
      powerCurrentAtom("gpuWatts"),
    );
    expect(liveSeriesAtom({ kind: "power", key: "gpuWatts" })).toBe(
      powerDrawSeriesAtom("gpuWatts"),
    );
  });
});

describe("series before any sample", () => {
  it("are one shared window of gaps, so waiting does not wake a subscriber", () => {
    const store = createStore();
    const cpu = store.get(cpuUsageSeriesAtom);
    const power = store.get(powerDrawSeriesAtom("cpuWatts"));
    const onPower = vi.fn();
    store.sub(powerDrawSeriesAtom("cpuWatts"), onPower);

    publish(store, { cpuUsage: 5 });

    expect(cpu).toEqual(paddedHistory());
    expect(power).toBe(store.get(powerDrawSeriesAtom("cpuWatts")));
    expect(onPower).not.toHaveBeenCalled();
    expect(store.get(cpuUsageSeriesAtom)).toEqual(paddedHistory(5));
  });
});

describe("latest per-core usage", () => {
  it("is null before a sample, then one entry per processor of the newest sample", () => {
    const store = createStore();
    expect(store.get(latestProcessorUsagesAtom)).toBeNull();

    publish(store, { processorsUsage: [1, 2, 3] });
    publish(store, { processorsUsage: [4, 5] });

    expect(store.get(latestProcessorUsagesAtom)).toEqual([4, 5]);
  });

  it("is an empty list for a sample without per-core data", () => {
    const store = createStore();

    publish(store, { processorsUsage: [] });

    expect(store.get(latestProcessorUsagesAtom)).toEqual([]);
  });

  it("keeps its reference, and does not notify, while no core moves", () => {
    const store = createStore();
    publish(store, { processorsUsage: [1, 2] });
    const latest = store.get(latestProcessorUsagesAtom);
    const onLatest = vi.fn();
    store.sub(latestProcessorUsagesAtom, onLatest);

    publish(store, { processorsUsage: [1, 2] });

    expect(store.get(latestProcessorUsagesAtom)).toBe(latest);
    expect(onLatest).not.toHaveBeenCalled();

    publish(store, { processorsUsage: [1, 3] });

    expect(onLatest).toHaveBeenCalledTimes(1);
  });
});

describe("compatibility atoms", () => {
  it("read empty before the first sample, like the atoms they replace", () => {
    const store = createStore();

    expect(store.get(cpuUsageHistoryAtom)).toEqual([]);
    expect(store.get(memoryUsageHistoryAtom)).toEqual([]);
    expect(store.get(processorsUsageHistoryAtom)).toEqual([]);
    expect(store.get(gpuUsageHistoriesAtom)).toEqual({});
    expect(store.get(powerDrawHistoryAtom)).toEqual({
      cpuWatts: [],
      gpuWatts: [],
      aneWatts: [],
      packageWatts: [],
    });
  });

  it("pad the histories once a sample has arrived and trim them to the window", () => {
    const store = createStore();
    publish(store, { cpuUsage: 10, memoryUsage: 60 });
    expect(store.get(cpuUsageHistoryAtom)).toEqual(paddedHistory(10));

    for (let i = 0; i < chartConfig.historyLengthSec + 5; i += 1) {
      publish(store, { cpuUsage: i, memoryUsage: i });
    }

    const history = store.get(memoryUsageHistoryAtom);
    expect(history).toHaveLength(chartConfig.historyLengthSec);
    expect(history.at(-1)).toBe(chartConfig.historyLengthSec + 4);
  });

  it("keep the same empty array between samples that add nothing", () => {
    const store = createStore();
    const history = store.get(powerDrawHistoryAtom);

    publish(store);

    expect(store.get(powerDrawHistoryAtom)).toBe(history);
  });

  it("do not wake a subscriber for readings that did not change", () => {
    const store = createStore();
    const gpus = [
      liveGpu("gpu-a", {
        gpuName: "GPU A",
        gpuUsage: 50,
        gpuTemperature: 60,
        gpuCoolerLevel: 30,
        gpuDedicatedMemoryUsageKb: 2048,
        gpuSource: "NVAPI",
      }),
    ];
    publish(store, { gpus, cpuPowerWatts: 10 });
    const subscribers = [
      gpuUsageSourcesAtom,
      gpuDedicatedMemoryKbMapAtom,
      gpuTempMapAtom,
      gpuFanSpeedMapAtom,
      powerDrawAtom,
    ].map((atom) => {
      const listener = vi.fn();
      store.sub(atom, listener);
      return listener;
    });
    const onHistories = vi.fn();
    store.sub(gpuUsageHistoriesAtom, onHistories);

    publish(store, { gpus, cpuPowerWatts: 10 });

    for (const listener of subscribers) {
      expect(listener).not.toHaveBeenCalled();
    }
    // A history moves with every sample.
    expect(onHistories).toHaveBeenCalledTimes(1);
  });

  it("wake a subscriber when its reading changes", () => {
    const store = createStore();
    publish(store, {
      gpus: [liveGpu("gpu-a", { gpuTemperature: 60 })],
      cpuPowerWatts: 10,
    });
    const onTemperature = vi.fn();
    const onPower = vi.fn();
    store.sub(gpuTempMapAtom, onTemperature);
    store.sub(powerDrawAtom, onPower);

    publish(store, {
      gpus: [liveGpu("gpu-a", { gpuTemperature: 61 })],
      cpuPowerWatts: 11,
    });

    expect(onTemperature).toHaveBeenCalledTimes(1);
    expect(onPower).toHaveBeenCalledTimes(1);
    expect(store.get(gpuTempMapAtom)).toEqual({
      [gpuA]: { name: "gpu-a", value: 61 },
    });
  });

  it("list only the readings the latest sample carried, so absence clears a value", () => {
    const store = createStore();
    publish(store, {
      gpus: [
        liveGpu("gpu-a", {
          gpuUsage: 50,
          gpuTemperature: 60,
          gpuCoolerLevel: 30,
          gpuDedicatedMemoryUsageKb: 2048,
          gpuSource: "NVAPI",
        }),
      ],
    });
    expect(store.get(gpuUsageSourcesAtom)).toEqual({ [gpuA]: "NVAPI" });
    expect(store.get(gpuDedicatedMemoryKbMapAtom)).toEqual({ [gpuA]: 2048 });

    publish(store, { gpus: [liveGpu("gpu-a", { gpuSource: "NVAPI" })] });

    expect(store.get(gpuTempMapAtom)).toEqual({});
    expect(store.get(gpuFanSpeedMapAtom)).toEqual({});
    expect(store.get(gpuDedicatedMemoryKbMapAtom)).toEqual({});
    expect(store.get(gpuUsageSourcesAtom)).toEqual({ [gpuA]: "NVAPI" });

    publish(store, { gpus: [] });

    expect(store.get(gpuUsageSourcesAtom)).toEqual({});
  });
});

describe("clearGpuTemperaturesAtom", () => {
  it("empties the temperature map and leaves the other readings", () => {
    const store = createStore();
    publish(store, {
      gpus: [liveGpu("gpu-a", { gpuUsage: 50, gpuTemperature: 60 })],
    });
    const onTemperature = vi.fn();
    store.sub(gpuTempMapAtom, onTemperature);

    store.set(clearGpuTemperaturesAtom);

    expect(store.get(gpuTempMapAtom)).toEqual({});
    expect(onTemperature).toHaveBeenCalledTimes(1);
    expect(store.get(gpuUsageHistoriesAtom)[gpuA]).toEqual(paddedHistory(50));
  });
});

describe("publishLiveSampleAtom", () => {
  it("resolves to the adapters the sample retired", () => {
    const store = createStore();
    publish(store, { gpus: [liveGpu("gpu-a", { gpuUsage: 50 })] });

    const results = [1, 2, 3].map(() => publish(store));

    expect(results.map((result) => result.retiredGpuIds)).toEqual([
      [],
      [],
      [gpuA],
    ]);
  });
});
