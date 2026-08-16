import { createStore } from "jotai";
import { describe, expect, it } from "vitest";
import {
  gpuDedicatedMemoryKbAtom,
  gpuDedicatedMemoryKbMapAtom,
  gpuNamesAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  gpuUsageSourceAtom,
  gpuUsageSourcesAtom,
  graphicUsageHistoryAtom,
  selectedGpuIdAtom,
} from "./chart";

/**
 * These atoms feed the classic Usage screen, the classic dashboard, and the
 * Monitor graph — surfaces that name an adapter elsewhere on the page. If they
 * resolved a selection differently from the GPU selectors, the page would
 * label one adapter and graph another.
 */
describe("derived GPU atoms", () => {
  const withSelection = (selected: string) => {
    const store = createStore();
    store.set(selectedGpuIdAtom, selected);
    store.set(gpuNamesAtom, {
      "nvapi:1": "GeForce RTX 4080",
      "pci:0:2:0": "UHD Graphics 770",
    });
    store.set(gpuUsageHistoriesAtom, { "nvapi:1": [70] });
    store.set(gpuUsageSourcesAtom, { "nvapi:1": "NVAPI" });
    store.set(gpuDedicatedMemoryKbMapAtom, { "nvapi:1": 4096 });
    store.set(gpuTempMapAtom, {
      "pci:0:2:0": { name: "UHD Graphics 770", value: 48 },
    });
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

    expect(store.get(graphicUsageHistoryAtom)).toEqual([70]);
    expect(store.get(gpuUsageSourceAtom)).toBe("NVAPI");
    expect(store.get(gpuDedicatedMemoryKbAtom)).toBe(4096);
  });

  it("falls back to the first reporting adapter when the selection is gone", () => {
    const store = withSelection("removed-gpu");

    expect(store.get(graphicUsageHistoryAtom)).toEqual([70]);
    expect(store.get(gpuUsageSourceAtom)).toBe("NVAPI");
  });

  it("has nothing to report before the first sample", () => {
    const store = createStore();

    expect(store.get(graphicUsageHistoryAtom)).toEqual([]);
    expect(store.get(gpuUsageSourceAtom)).toBeNull();
    expect(store.get(gpuDedicatedMemoryKbAtom)).toBeNull();
  });
});
