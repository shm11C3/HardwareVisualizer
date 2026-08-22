import { describe, expect, it } from "vitest";
import {
  asLiveGpuId,
  findInventoryGpu,
  type GpuLiveMaps,
  getEffectiveGpuId,
  hasNoLiveGpuReadings,
  type LiveGpuId,
  listGpuAdapters,
  toLiveGpuId,
} from "./gpuIdentity";

const inventory = [
  { id: "12345", name: "NVIDIA GeForce RTX 4080" },
  { id: "67890", name: "Intel UHD Graphics 770" },
];

/** The live name map, as the event listener builds it from each sample. */
const names = (...pairs: [string, string][]) =>
  Object.fromEntries(
    pairs.map(([id, name]) => [asLiveGpuId(id), name]),
  ) as Record<LiveGpuId, string>;

/** Test seeds mint ids the way the event listener does. */
const id = asLiveGpuId;

/**
 * Seeds take plain string keys and mint them here, the same way the event
 * listener mints ids at the payload boundary.
 */
const live = (
  maps: {
    usageHistories?: Record<string, (number | null)[]>;
    temperatures?: Record<string, { name: string; value: number }>;
    fanSpeeds?: Record<string, { name: string; value: number }>;
    dedicatedMemoryKb?: Record<string, number | null>;
  } = {},
): GpuLiveMaps =>
  ({
    usageHistories: {},
    temperatures: {},
    fanSpeeds: {},
    dedicatedMemoryKb: {},
    ...maps,
  }) as GpuLiveMaps;

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
      getEffectiveGpuId(
        id("gpu-2"),
        live({ usageHistories: { "gpu-1": [20] } }),
        [id("gpu-1"), id("gpu-2")],
      ),
    ).toBe("gpu-2");
  });

  it("drops a selection whose adapter is gone instead of showing nothing", () => {
    expect(
      getEffectiveGpuId(
        id("removed"),
        live({ usageHistories: { "gpu-1": [20] } }),
        [id("gpu-1")],
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
    expect(hasNoLiveGpuReadings(id("gpu-2"), live())).toBe(false);
  });

  it("reports an adapter that stayed silent while another one answered", () => {
    expect(
      hasNoLiveGpuReadings(
        id("gpu-2"),
        live({ usageHistories: { "gpu-1": [20] } }),
      ),
    ).toBe(true);
  });

  it("counts a temperature-only adapter as reporting", () => {
    expect(
      hasNoLiveGpuReadings(
        id("gpu-2"),
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
        id("gpu-2"),
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
        id("gpu-2"),
        live({
          usageHistories: { "gpu-1": [20] },
          dedicatedMemoryKb: { "gpu-2": 1_048_576 },
        }),
      ),
    ).toBe(false);
  });
});

describe("hasNoLiveGpuReadings with an all-null sample", () => {
  it("explains a lone adapter that reports only its own name", () => {
    // An Intel GPU whose PDH usage query fails: the sample arrives, the name
    // is recorded, and all four value maps stay empty forever. Treating that
    // as "not measured yet" would blank the explanation permanently.
    expect(
      hasNoLiveGpuReadings(id("pdh:UHD Graphics"), live({}), [
        id("pdh:UHD Graphics"),
      ]),
    ).toBe(true);
  });

  it("still says nothing before any adapter is detected", () => {
    expect(hasNoLiveGpuReadings(id("pdh:UHD Graphics"), live({}), [])).toBe(
      false,
    );
  });
});

describe("findInventoryGpu", () => {
  it("resolves a live id to its inventory entry through the shared name", () => {
    // Selecting the second adapter on Performance writes a live id. Resolving
    // it by id alone would land on the first inventory entry and pair its
    // name with the second adapter's readings.
    expect(
      findInventoryGpu(
        inventory,
        names(["pci:0:2:0", "Intel UHD Graphics 770"]),
        id("pci:0:2:0"),
      ),
    ).toEqual(inventory[1]);
  });

  it("still accepts an id the inventory itself uses", () => {
    expect(findInventoryGpu(inventory, {}, id("67890"))).toEqual(inventory[1]);
  });

  it("refuses the join when the name picks out more than one entry", () => {
    const twins = [
      { id: "a", name: "NVIDIA GeForce RTX 4090" },
      { id: "b", name: "NVIDIA GeForce RTX 4090" },
    ];

    expect(
      findInventoryGpu(
        twins,
        names(["nvapi:1", "NVIDIA GeForce RTX 4090"]),
        id("nvapi:1"),
      ),
    ).toBeUndefined();
  });

  it("refuses rather than pairing by position when both sides have twins", () => {
    // Positional pairing would look plausible and be a guess: the inventory's
    // enumeration order and the stream's are different enumerations.
    const twins = [
      { id: "inv-a", name: "NVIDIA GeForce RTX 4090" },
      { id: "inv-b", name: "NVIDIA GeForce RTX 4090" },
    ];

    expect(
      findInventoryGpu(
        twins,
        names(
          ["nvapi:1", "NVIDIA GeForce RTX 4090"],
          ["nvapi:2", "NVIDIA GeForce RTX 4090"],
        ),
        id("nvapi:2"),
      ),
    ).toBeUndefined();
  });

  it("has nothing to resolve without a selection", () => {
    expect(findInventoryGpu(inventory, {}, null)).toBeUndefined();
  });
});

describe("toLiveGpuId", () => {
  it("writes the shared selection in the namespace readings are keyed by", () => {
    expect(
      toLiveGpuId(inventory[1], names(["pci:0:2:0", "Intel UHD Graphics 770"])),
    ).toBe("pci:0:2:0");
  });

  it("keeps the inventory id when no live adapter reports that name", () => {
    expect(toLiveGpuId(inventory[1], {})).toBe("67890");
  });

  it("keeps the inventory id before any sample names an adapter", () => {
    // The classic card is available before the first monitor sample. The id
    // it returns here cannot address readings, so the card reconciles it once
    // the stream names the adapter.
    expect(toLiveGpuId(inventory[1], {})).toBe("67890");
  });

  it("keeps the inventory id when the name is ambiguous", () => {
    expect(
      toLiveGpuId(
        { id: "a", name: "NVIDIA GeForce RTX 4090" },
        names(
          ["nvapi:1", "NVIDIA GeForce RTX 4090"],
          ["nvapi:2", "NVIDIA GeForce RTX 4090"],
        ),
      ),
    ).toBe("a");
  });
});
