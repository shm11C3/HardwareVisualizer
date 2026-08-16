import { describe, expect, it } from "vitest";
import type { GraphicInfo } from "@/rspc/bindings";
import {
  type GpuLiveMaps,
  getEffectiveGpuId,
  hasNoLiveGpuReadings,
  listGpuAdapters,
} from "./gpuIdentity";

const gpu = (id: string, name: string): GraphicInfo => ({
  id,
  name,
  vendorName: "Vendor",
  clock: 0,
  memorySize: "8 GB",
  memorySizeDedicated: "8 GB",
  coreCount: null,
});

const live = (maps: Partial<GpuLiveMaps> = {}): GpuLiveMaps => ({
  usageHistories: {},
  temperatures: {},
  fanSpeeds: {},
  dedicatedMemoryKb: {},
  ...maps,
});

describe("listGpuAdapters", () => {
  it("represents every detected adapter even when only one reports values", () => {
    const adapters = listGpuAdapters(
      [gpu("gpu-1", "NVIDIA GeForce RTX 4080"), gpu("gpu-2", "Intel UHD 770")],
      live({ usageHistories: { "gpu-1": [20] } }),
    );

    expect(adapters.map((adapter) => adapter.id)).toEqual(["gpu-1", "gpu-2"]);
  });

  it("keeps an adapter that only exists in the live maps, so no reading is ownerless", () => {
    const adapters = listGpuAdapters(
      [gpu("gpu-1", "Radeon 780M")],
      live({ temperatures: { "gpu-9": { name: "Late GPU", value: 40 } } }),
    );

    expect(adapters).toEqual([
      { id: "gpu-1", name: "Radeon 780M", label: "Radeon 780M" },
      { id: "gpu-9", name: "Late GPU", label: "Late GPU" },
    ]);
  });

  it("names an adapter from the fan map when the static fetch has not answered", () => {
    // The static fetch can be slow or fail outright; the sensor maps carry the
    // platform's own name, so an id never has to be shown raw when one exists.
    const adapters = listGpuAdapters(
      null,
      live({
        usageHistories: { "GPU-{8ee6}": [30] },
        fanSpeeds: { "GPU-{8ee6}": { name: "Radeon RX 7900 XT", value: 55 } },
      }),
    );

    expect(adapters).toEqual([
      {
        id: "GPU-{8ee6}",
        name: "Radeon RX 7900 XT",
        label: "Radeon RX 7900 XT",
      },
    ]);
  });

  it("falls back to the id only when no map names the adapter at all", () => {
    const adapters = listGpuAdapters(
      null,
      live({ usageHistories: { "GPU-{8ee6}": [30] } }),
    );

    expect(adapters[0].label).toBe("GPU-{8ee6}");
  });

  it("drops the vendor word that every adapter of a brand repeats", () => {
    const adapters = listGpuAdapters(
      [
        gpu("gpu-1", "NVIDIA GeForce RTX 4080"),
        gpu("gpu-2", "Intel(R) UHD Graphics 770"),
      ],
      live(),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "GeForce RTX 4080",
      "UHD Graphics 770",
    ]);
  });

  it("falls back to full names rather than showing two identical labels", () => {
    const adapters = listGpuAdapters(
      [gpu("gpu-1", "NVIDIA RTX A2000"), gpu("gpu-2", "AMD RTX A2000")],
      live(),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "NVIDIA RTX A2000",
      "AMD RTX A2000",
    ]);
  });

  it("keeps full names when trimming the shared prefix would collide", () => {
    // Two identical cards: shortening cannot tell them apart, so it must not
    // pretend to by showing the same short label twice.
    const adapters = listGpuAdapters(
      [
        gpu("gpu-1", "NVIDIA GeForce RTX 4090"),
        gpu("gpu-2", "NVIDIA GeForce RTX 4090"),
      ],
      live(),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "NVIDIA GeForce RTX 4090",
      "NVIDIA GeForce RTX 4090",
    ]);
  });

  it("drops the leading words two adapters share, since a narrow card truncates the rest away", () => {
    const adapters = listGpuAdapters(
      [
        gpu("gpu-1", "NVIDIA GeForce RTX 4080"),
        gpu("gpu-2", "NVIDIA GeForce RTX 4060"),
      ],
      live(),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual(["4080", "4060"]);
    // The full name is still what the control reports for a tooltip.
    expect(adapters[0].name).toBe("NVIDIA GeForce RTX 4080");
  });

  it("never consumes a whole name when one adapter's name prefixes another's", () => {
    const adapters = listGpuAdapters(
      [gpu("gpu-1", "Radeon Graphics"), gpu("gpu-2", "Radeon Graphics Pro")],
      live(),
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "Graphics",
      "Graphics Pro",
    ]);
  });

  it("keeps a name that is nothing but a vendor word", () => {
    expect(listGpuAdapters([gpu("gpu-1", "Apple")], live())[0].label).toBe(
      "Apple",
    );
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
