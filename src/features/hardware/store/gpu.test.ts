import { createStore } from "jotai";
import { describe, expect, it, vi } from "vitest";
import { asLiveGpuId, type LiveGpuId } from "@/features/hardware/gpuIdentity";
import {
  liveGpu,
  liveSample,
  paddedHistory,
} from "@/features/hardware/live/liveSamples.testHelpers";
import {
  effectiveGpuAdapterAtom,
  effectiveGpuIdAtom,
  effectiveGpuUsageCurrentAtom,
  effectiveGpuUsageSeriesAtom,
  gpuAdaptersAtom,
  gpuDedicatedMemoryKbAtom,
  gpuFanSpeedValueAtom,
  gpuHasNoReadingsAtom,
  gpuNamesAtom,
  gpuTemperatureValueAtom,
  gpuUsageSourceAtom,
  graphicUsageHistoryAtom,
} from "@/features/hardware/store/gpu";
import { publishLiveSampleAtom } from "@/features/hardware/store/liveMetrics";
import { selectedGpuIdAtom } from "@/features/hardware/store/selection";

/**
 * These atoms feed the classic Usage screen, the classic dashboard, and the
 * Monitor graph — surfaces that name an adapter elsewhere on the page. If they
 * resolved a selection differently from the GPU selectors, the page would
 * label one adapter and graph another.
 */
describe("derived GPU atoms", () => {
  /** Seeds mint live ids the way the event listener does at the boundary. */
  const liveMap = <T>(map: Record<string, T>) =>
    map as unknown as Record<LiveGpuId, T>;

  const withSelection = (selected: string) => {
    const store = createStore();
    store.set(selectedGpuIdAtom, asLiveGpuId(selected));
    store.set(
      gpuNamesAtom,
      liveMap({
        "nvapi:1": "GeForce RTX 4080",
        "pci:0:2:0": "UHD Graphics 770",
      }),
    );
    store.set(
      publishLiveSampleAtom,
      liveSample({
        gpus: [
          liveGpu("nvapi:1", {
            gpuUsage: 70,
            gpuSource: "NVAPI",
            gpuDedicatedMemoryUsageKb: 4096,
          }),
          liveGpu("pci:0:2:0", {
            gpuName: "UHD Graphics 770",
            gpuTemperature: 48,
          }),
        ],
      }),
      0,
    );
    return store;
  };

  it("reports nothing for a selected adapter that has no usage of its own", () => {
    // The iGPU reports a temperature but no usage. Falling back to the first
    // history would graph the discrete card under the integrated one's name.
    const store = withSelection("pci:0:2:0");

    expect(store.get(graphicUsageHistoryAtom)).toEqual([]);
    expect(store.get(gpuUsageSourceAtom)).toBeNull();
    expect(store.get(gpuDedicatedMemoryKbAtom)).toBeNull();
  });

  it("resolves a selection that does report", () => {
    const store = withSelection("nvapi:1");

    expect(store.get(graphicUsageHistoryAtom)).toEqual(paddedHistory(70));
    expect(store.get(gpuUsageSourceAtom)).toBe("NVAPI");
    expect(store.get(gpuDedicatedMemoryKbAtom)).toBe(4096);
  });

  it("falls back to the first reporting adapter when the selection is gone", () => {
    const store = withSelection("removed-gpu");

    expect(store.get(graphicUsageHistoryAtom)).toEqual(paddedHistory(70));
    expect(store.get(gpuUsageSourceAtom)).toBe("NVAPI");
  });

  it("has nothing to report before the first sample", () => {
    const store = createStore();

    expect(store.get(graphicUsageHistoryAtom)).toEqual([]);
    expect(store.get(gpuUsageSourceAtom)).toBeNull();
    expect(store.get(gpuDedicatedMemoryKbAtom)).toBeNull();
  });

  it("reads the effective adapter's live channel, padded, without resolving an id", () => {
    const store = withSelection("nvapi:1");

    expect(store.get(effectiveGpuUsageSeriesAtom)).toEqual(paddedHistory(70));
    expect(store.get(effectiveGpuUsageCurrentAtom)).toBe(70);
  });

  it("reads as all gaps, and no current value, for an adapter without usage", () => {
    const store = withSelection("pci:0:2:0");

    expect(store.get(effectiveGpuUsageSeriesAtom)).toEqual(paddedHistory());
    expect(store.get(effectiveGpuUsageCurrentAtom)).toBeNull();
  });

  it("reads as all gaps before the first sample, as one shared array", () => {
    const store = createStore();
    const empty = store.get(effectiveGpuUsageSeriesAtom);

    expect(empty).toEqual(paddedHistory());
    expect(store.get(effectiveGpuUsageCurrentAtom)).toBeNull();

    store.set(publishLiveSampleAtom, liveSample(), 0);

    expect(store.get(effectiveGpuUsageSeriesAtom)).toBe(empty);
  });

  it("follows the selection to another adapter's channel", () => {
    const store = withSelection("nvapi:1");
    const onCurrent = vi.fn();
    store.sub(effectiveGpuUsageCurrentAtom, onCurrent);

    store.set(selectedGpuIdAtom, asLiveGpuId("pci:0:2:0"));

    expect(onCurrent).toHaveBeenCalledTimes(1);
    expect(store.get(effectiveGpuUsageCurrentAtom)).toBeNull();
  });

  it("does not wake a current-value subscriber for a sample that leaves it unchanged", () => {
    const store = withSelection("nvapi:1");
    const onCurrent = vi.fn();
    store.sub(effectiveGpuUsageCurrentAtom, onCurrent);

    store.set(
      publishLiveSampleAtom,
      liveSample({ gpus: [liveGpu("nvapi:1", { gpuUsage: 70 })] }),
      0,
    );

    expect(onCurrent).not.toHaveBeenCalled();
  });
});

/**
 * The adapter list and the effective-adapter answers change when the hardware
 * does, not when a reading does. Screens subscribe to them to avoid
 * re-rendering once a second (subscription granularity, #1638).
 */
describe("GPU identity atoms stay stable between samples", () => {
  const liveMap = <T>(map: Record<string, T>) =>
    map as unknown as Record<LiveGpuId, T>;

  const seeded = () => {
    const store = createStore();
    store.set(selectedGpuIdAtom, asLiveGpuId("nvapi:1"));
    store.set(
      gpuNamesAtom,
      liveMap({
        "nvapi:1": "GeForce RTX 4080",
        "pci:0:2:0": "UHD Graphics 770",
      }),
    );
    store.set(
      publishLiveSampleAtom,
      liveSample({
        gpus: [
          liveGpu("nvapi:1", {
            gpuName: "GeForce RTX 4080",
            gpuUsage: 70,
            gpuTemperature: 60,
          }),
        ],
      }),
      0,
    );
    return store;
  };

  it("keeps the adapter list reference while only readings change", () => {
    const store = seeded();
    const adapters = store.get(gpuAdaptersAtom);
    const effective = store.get(effectiveGpuAdapterAtom);
    expect(adapters.map((adapter) => adapter.id)).toEqual([
      "nvapi:1",
      "pci:0:2:0",
    ]);

    store.set(
      publishLiveSampleAtom,
      liveSample({
        gpus: [
          liveGpu("nvapi:1", {
            gpuName: "GeForce RTX 4080",
            gpuUsage: 71,
            gpuTemperature: 61,
          }),
        ],
      }),
      0,
    );

    expect(store.get(gpuAdaptersAtom)).toBe(adapters);
    expect(store.get(effectiveGpuAdapterAtom)).toBe(effective);
  });

  it("does not notify an adapter-list subscriber when only readings change", () => {
    const store = seeded();
    const onAdapters = vi.fn();
    const onEffective = vi.fn();
    const onNoReadings = vi.fn();
    store.sub(gpuAdaptersAtom, onAdapters);
    store.sub(effectiveGpuIdAtom, onEffective);
    store.sub(gpuHasNoReadingsAtom, onNoReadings);

    store.set(
      publishLiveSampleAtom,
      liveSample({
        gpus: [
          liveGpu("nvapi:1", {
            gpuName: "GeForce RTX 4080",
            gpuUsage: 71,
            gpuTemperature: 61,
          }),
        ],
      }),
      0,
    );

    expect(onAdapters).not.toHaveBeenCalled();
    expect(onEffective).not.toHaveBeenCalled();
    expect(onNoReadings).not.toHaveBeenCalled();
  });

  it("publishes a new list when an adapter appears or is renamed", () => {
    const store = seeded();
    const adapters = store.get(gpuAdaptersAtom);

    store.set(
      gpuNamesAtom,
      liveMap({
        "nvapi:1": "GeForce RTX 4080",
        "pci:0:2:0": "UHD Graphics 770",
        "pci:0:3:0": "Radeon",
      }),
    );
    const grown = store.get(gpuAdaptersAtom);
    expect(grown).not.toBe(adapters);
    expect(grown).toHaveLength(3);

    store.set(
      gpuNamesAtom,
      liveMap({
        "nvapi:1": "GeForce RTX 4090",
        "pci:0:2:0": "UHD Graphics 770",
        "pci:0:3:0": "Radeon",
      }),
    );
    expect(store.get(gpuAdaptersAtom)).not.toBe(grown);
    expect(store.get(effectiveGpuAdapterAtom)?.name).toBe("GeForce RTX 4090");
  });

  it("keeps naming an adapter that only a sensor map knows about", () => {
    const store = createStore();
    store.set(
      publishLiveSampleAtom,
      liveSample({
        gpus: [
          liveGpu("nvapi:1", {
            gpuName: "GeForce RTX 4080",
            gpuTemperature: 60,
          }),
        ],
      }),
      0,
    );

    expect(store.get(gpuAdaptersAtom)).toEqual([
      {
        id: "nvapi:1",
        name: "GeForce RTX 4080",
        label: "GeForce RTX 4080",
        isNameAmbiguous: false,
      },
    ]);
  });

  it("reports no readings only for a detected adapter that has none", () => {
    const store = seeded();
    expect(store.get(gpuHasNoReadingsAtom)).toBe(false);

    store.set(selectedGpuIdAtom, asLiveGpuId("pci:0:2:0"));
    expect(store.get(effectiveGpuIdAtom)).toBe("pci:0:2:0");
    expect(store.get(gpuHasNoReadingsAtom)).toBe(true);
  });

  it("is silent before the first sample", () => {
    const store = createStore();

    expect(store.get(gpuAdaptersAtom)).toEqual([]);
    expect(store.get(effectiveGpuIdAtom)).toBeUndefined();
    expect(store.get(effectiveGpuAdapterAtom)).toBeUndefined();
    expect(store.get(gpuHasNoReadingsAtom)).toBe(false);
  });

  it("hands out the effective adapter's own temperature and fan speed", () => {
    const store = seeded();
    store.set(
      publishLiveSampleAtom,
      liveSample({
        gpus: [
          liveGpu("nvapi:1", {
            gpuName: "GeForce RTX 4080",
            gpuUsage: 70,
            gpuTemperature: 60,
            gpuCoolerLevel: 42,
          }),
        ],
      }),
      0,
    );
    expect(store.get(gpuTemperatureValueAtom)).toBe(60);
    expect(store.get(gpuFanSpeedValueAtom)).toBe(42);

    store.set(selectedGpuIdAtom, asLiveGpuId("pci:0:2:0"));
    expect(store.get(gpuTemperatureValueAtom)).toBeNull();
    expect(store.get(gpuFanSpeedValueAtom)).toBeNull();
  });

  it("returns one shared empty history, not a new array per sample", () => {
    const store = seeded();
    store.set(selectedGpuIdAtom, asLiveGpuId("pci:0:2:0"));
    const empty = store.get(graphicUsageHistoryAtom);

    store.set(
      publishLiveSampleAtom,
      liveSample({
        gpus: [liveGpu("nvapi:1", { gpuUsage: 71, gpuTemperature: 60 })],
      }),
      0,
    );

    expect(empty).toEqual([]);
    expect(store.get(graphicUsageHistoryAtom)).toBe(empty);
  });
});
