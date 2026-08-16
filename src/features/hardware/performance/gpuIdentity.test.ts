import { describe, expect, it } from "vitest";
import type { GraphicInfo } from "@/rspc/bindings";
import {
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

describe("listGpuAdapters", () => {
  it("represents every detected adapter even when only one reports values", () => {
    const adapters = listGpuAdapters(
      [gpu("gpu-1", "NVIDIA GeForce RTX 4080"), gpu("gpu-2", "Intel UHD 770")],
      {},
      ["gpu-1"],
    );

    expect(adapters.map((adapter) => adapter.id)).toEqual(["gpu-1", "gpu-2"]);
  });

  it("keeps an adapter that only exists in the live maps, so no reading is ownerless", () => {
    const adapters = listGpuAdapters([gpu("gpu-1", "Radeon 780M")], {
      "gpu-9": { name: "Late GPU" },
    });

    expect(adapters).toEqual([
      { id: "gpu-1", name: "Radeon 780M", label: "Radeon 780M" },
      { id: "gpu-9", name: "Late GPU", label: "Late GPU" },
    ]);
  });

  it("drops the vendor word that every adapter of a brand repeats", () => {
    const adapters = listGpuAdapters(
      [
        gpu("gpu-1", "NVIDIA GeForce RTX 4080"),
        gpu("gpu-2", "Intel(R) UHD Graphics 770"),
      ],
      {},
      [],
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "GeForce RTX 4080",
      "UHD Graphics 770",
    ]);
  });

  it("falls back to full names rather than showing two identical labels", () => {
    const adapters = listGpuAdapters(
      [gpu("gpu-1", "NVIDIA RTX A2000"), gpu("gpu-2", "AMD RTX A2000")],
      {},
      [],
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "NVIDIA RTX A2000",
      "AMD RTX A2000",
    ]);
  });

  it("drops the leading words two adapters share, since a narrow card truncates the rest away", () => {
    const adapters = listGpuAdapters(
      [
        gpu("gpu-1", "NVIDIA GeForce RTX 4080"),
        gpu("gpu-2", "NVIDIA GeForce RTX 4060"),
      ],
      {},
      [],
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual(["4080", "4060"]);
    // The full name is still what the control reports for a tooltip.
    expect(adapters[0].name).toBe("NVIDIA GeForce RTX 4080");
  });

  it("never consumes a whole name when one adapter's name prefixes another's", () => {
    const adapters = listGpuAdapters(
      [gpu("gpu-1", "Radeon Graphics"), gpu("gpu-2", "Radeon Graphics Pro")],
      {},
      [],
    );

    expect(adapters.map((adapter) => adapter.label)).toEqual([
      "Graphics",
      "Graphics Pro",
    ]);
  });

  it("keeps a name that is nothing but a vendor word", () => {
    expect(listGpuAdapters([gpu("gpu-1", "Apple")], {}, [])[0].label).toBe(
      "Apple",
    );
  });
});

describe("getEffectiveGpuId", () => {
  it("keeps an explicit selection that reports nothing yet", () => {
    expect(
      getEffectiveGpuId("gpu-2", { "gpu-1": [20] }, {}, ["gpu-1", "gpu-2"]),
    ).toBe("gpu-2");
  });

  it("drops a selection whose adapter is gone instead of showing nothing", () => {
    expect(getEffectiveGpuId("removed", { "gpu-1": [20] }, {}, ["gpu-1"])).toBe(
      "gpu-1",
    );
  });

  it("falls back to a temperature-only adapter when no usage is reported", () => {
    expect(
      getEffectiveGpuId(null, {}, { "gpu-1": { name: "GPU 1", value: 40 } }),
    ).toBe("gpu-1");
  });
});

describe("hasNoLiveGpuReadings", () => {
  it("stays silent before the first sample, which is 'not yet' rather than 'unavailable'", () => {
    expect(hasNoLiveGpuReadings("gpu-2", {}, {})).toBe(false);
  });

  it("reports an adapter that stayed silent while another one answered", () => {
    expect(hasNoLiveGpuReadings("gpu-2", { "gpu-1": [20] }, {})).toBe(true);
  });

  it("counts a temperature-only adapter as reporting", () => {
    expect(
      hasNoLiveGpuReadings(
        "gpu-2",
        { "gpu-1": [20] },
        { "gpu-2": { name: "GPU 2", value: 51 } },
      ),
    ).toBe(false);
  });
});
