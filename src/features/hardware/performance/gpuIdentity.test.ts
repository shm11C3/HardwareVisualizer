import { describe, expect, it } from "vitest";
import {
  type GpuLiveMaps,
  getEffectiveGpuId,
  hasNoLiveGpuReadings,
  listGpuAdapters,
} from "./gpuIdentity";

/** The live name map, as the event listener builds it from each sample. */
const names = (...pairs: [string, string][]) => Object.fromEntries(pairs);

const live = (maps: Partial<GpuLiveMaps> = {}): GpuLiveMaps => ({
  usageHistories: {},
  temperatures: {},
  fanSpeeds: {},
  dedicatedMemoryKb: {},
  ...maps,
});

describe("listGpuAdapters", () => {
  it("lists one adapter per live id, never unioned with the inventory namespace", () => {
    // Windows NVIDIA: the inventory reports "12345", sampling reports
    // "nvapi:12345". Unioning them would render one card as two adapters and
    // declare the inventory half silent.
    const adapters = listGpuAdapters(
      names(["nvapi:12345", "NVIDIA GeForce RTX 4080"]),
      live({ usageHistories: { "nvapi:12345": [30] } }),
    );

    expect(adapters).toEqual([
      {
        id: "nvapi:12345",
        name: "NVIDIA GeForce RTX 4080",
        label: "GeForce RTX 4080",
        isNameAmbiguous: false,
      },
    ]);
  });

  it("covers every live map, so a fan-only adapter is still listed", () => {
    const adapters = listGpuAdapters(
      names(["pci:1:0:0", "Radeon RX 7900 XT"]),
      live({
        usageHistories: { "nvapi:1": [30] },
        fanSpeeds: { "pci:1:0:0": { name: "Radeon RX 7900 XT", value: 55 } },
      }),
    );

    // Named adapters come first in payload order, then any id that only a
    // value map knows about.
    expect(adapters.map((adapter) => adapter.id)).toEqual([
      "pci:1:0:0",
      "nvapi:1",
    ]);
  });

  it("names an adapter from the sensor map when the name map has not caught up", () => {
    const adapters = listGpuAdapters(
      {},
      live({
        temperatures: { "iokit:M3": { name: "Apple M3 Max", value: 51 } },
      }),
    );

    expect(adapters[0].name).toBe("Apple M3 Max");
  });

  it("falls back to the id only when no source names the adapter", () => {
    const adapters = listGpuAdapters(
      {},
      live({ usageHistories: { "pdh:Unknown": [30] } }),
    );

    expect(adapters[0].label).toBe("pdh:Unknown");
  });

  it("drops the vendor word that every adapter of a brand repeats", () => {
    const adapters = listGpuAdapters(
      names(
        ["gpu-1", "NVIDIA GeForce RTX 4080"],
        ["gpu-2", "Intel(R) UHD Graphics 770"],
      ),
      live({ usageHistories: { "gpu-1": [20], "gpu-2": [5] } }),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "GeForce RTX 4080",
      "UHD Graphics 770",
    ]);
  });

  it("falls back to full names rather than showing two identical labels", () => {
    const adapters = listGpuAdapters(
      names(["gpu-1", "NVIDIA RTX A2000"], ["gpu-2", "AMD RTX A2000"]),
      live({ usageHistories: { "gpu-1": [20], "gpu-2": [5] } }),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "NVIDIA RTX A2000",
      "AMD RTX A2000",
    ]);
  });

  it("adds an ordinal when the platform reports the same name twice", () => {
    // Two identical cards. Neither shortening nor the full name can tell them
    // apart, and two controls that read identically are not a choice.
    const adapters = listGpuAdapters(
      names(
        ["gpu-1", "NVIDIA GeForce RTX 4090"],
        ["gpu-2", "NVIDIA GeForce RTX 4090"],
      ),
      live({ usageHistories: { "gpu-1": [20], "gpu-2": [5] } }),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "GeForce RTX 4090 #1",
      "GeForce RTX 4090 #2",
    ]);
    // Anything that keys on the name has to refuse it here.
    expect(adapters.every((adapter) => adapter.isNameAmbiguous)).toBe(true);
  });

  it("leaves distinct adapters unmarked, so name joins stay allowed", () => {
    const adapters = listGpuAdapters(
      names(
        ["gpu-1", "NVIDIA GeForce RTX 4080"],
        ["gpu-2", "Intel UHD Graphics 770"],
      ),
      live({ usageHistories: { "gpu-1": [20], "gpu-2": [5] } }),
    );

    expect(adapters.every((adapter) => adapter.isNameAmbiguous)).toBe(false);
  });

  it("drops the leading words two adapters share, since a narrow card truncates the rest away", () => {
    const adapters = listGpuAdapters(
      names(
        ["gpu-1", "NVIDIA GeForce RTX 4080"],
        ["gpu-2", "NVIDIA GeForce RTX 4060"],
      ),
      live({ usageHistories: { "gpu-1": [20], "gpu-2": [5] } }),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual(["4080", "4060"]);
    // The full name is still what the control reports for a tooltip.
    expect(adapters[0].name).toBe("NVIDIA GeForce RTX 4080");
  });

  it("never consumes a whole name when one adapter's name prefixes another's", () => {
    const adapters = listGpuAdapters(
      names(["gpu-1", "Radeon Graphics"], ["gpu-2", "Radeon Graphics Pro"]),
      live({ usageHistories: { "gpu-1": [20], "gpu-2": [5] } }),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "Graphics",
      "Graphics Pro",
    ]);
  });

  it("keeps a name that is nothing but a vendor word", () => {
    const adapters = listGpuAdapters(
      names(["gpu-1", "Apple"]),
      live({ usageHistories: { "gpu-1": [20] } }),
    );

    expect(adapters[0].label).toBe("Apple");
  });
});

describe("getEffectiveGpuId", () => {
  it("keeps an explicit selection that reports nothing yet", () => {
    expect(
      getEffectiveGpuId("gpu-2", live({ usageHistories: { "gpu-1": [20] } }), [
        "gpu-1",
        "gpu-2",
      ]),
    ).toBe("gpu-2");
  });

  it("drops a selection whose adapter is gone instead of showing nothing", () => {
    expect(
      getEffectiveGpuId(
        "removed",
        live({ usageHistories: { "gpu-1": [20] } }),
        ["gpu-1"],
      ),
    ).toBe("gpu-1");
  });

  it("falls back through every map, so a fan-only adapter is still reachable", () => {
    expect(
      getEffectiveGpuId(
        null,
        live({ fanSpeeds: { "gpu-1": { name: "GPU 1", value: 55 } } }),
      ),
    ).toBe("gpu-1");
  });

  it("has no effective GPU before anything reports", () => {
    expect(getEffectiveGpuId(null, live())).toBeUndefined();
  });
});

describe("hasNoLiveGpuReadings", () => {
  it("stays silent before the first sample, which is 'not yet' rather than 'unavailable'", () => {
    expect(hasNoLiveGpuReadings("gpu-2", live())).toBe(false);
  });

  it("reports an adapter that stayed silent while another one answered", () => {
    expect(
      hasNoLiveGpuReadings(
        "gpu-2",
        live({ usageHistories: { "gpu-1": [20] } }),
      ),
    ).toBe(true);
  });

  it("counts a temperature-only adapter as reporting", () => {
    expect(
      hasNoLiveGpuReadings(
        "gpu-2",
        live({
          usageHistories: { "gpu-1": [20] },
          temperatures: { "gpu-2": { name: "GPU 2", value: 51 } },
        }),
      ),
    ).toBe(false);
  });

  it("counts a fan-only adapter as reporting, so its fan speed is not called absent", () => {
    expect(
      hasNoLiveGpuReadings(
        "gpu-2",
        live({
          usageHistories: { "gpu-1": [20] },
          fanSpeeds: { "gpu-2": { name: "GPU 2", value: 40 } },
        }),
      ),
    ).toBe(false);
  });

  it("counts a VRAM-only adapter as reporting", () => {
    expect(
      hasNoLiveGpuReadings(
        "gpu-2",
        live({
          usageHistories: { "gpu-1": [20] },
          dedicatedMemoryKb: { "gpu-2": 1_048_576 },
        }),
      ),
    ).toBe(false);
  });
});
