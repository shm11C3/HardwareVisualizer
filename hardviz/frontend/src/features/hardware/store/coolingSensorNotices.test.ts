import { createStore } from "jotai";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  coolingInsightPeriodAtom,
  coolingInsightPeriodHydratedAtom,
} from "@/features/hardware/store/coolingPeriod";
import {
  coolingAmbientSourcesAtom,
  coolingFanNoticeAtom,
  coolingPowerNoticeAtom,
} from "@/features/hardware/store/coolingSensorNotices";
import { cpuPowerSupportAtom } from "@/features/hardware/store/power";
import { motherboardFanSupportAtom } from "@/features/hardware/store/sensors";

const mocks = vi.hoisted(() => ({
  getDataArchiveSeries: vi.fn(),
  getFanArchiveSeries: vi.fn(),
  getAmbientArchiveSeries: vi.fn(),
  getCoolingTrend: vi.fn(),
  getCoolingFanTrend: vi.fn(),
}));

vi.mock("@/rspc/bindings", () => ({ commands: mocks }));

const flush = async () => {
  for (let i = 0; i < 10; i++) {
    await Promise.resolve();
  }
};

const ok = <T>(data: T) => ({ status: "ok" as const, data });

/** Temperature and usage were recorded; power never was. */
const recordedWithoutPower = (hardwareType: string) =>
  ok(
    hardwareType === "cpuPower"
      ? []
      : [{ timestamp: 1_700_000_000_000, value: 40 }],
  );

const mountNotices = (period: "7d" | "90d") => {
  const store = createStore();
  store.set(coolingInsightPeriodAtom, period);
  store.set(coolingInsightPeriodHydratedAtom, true);
  store.set(cpuPowerSupportAtom, "supported");
  store.set(motherboardFanSupportAtom, "unsupported");
  for (const atom of [
    coolingPowerNoticeAtom,
    coolingFanNoticeAtom,
    coolingAmbientSourcesAtom,
  ]) {
    store.sub(atom, () => {});
  }
  return store;
};

describe("Cooling sensor notices", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.getDataArchiveSeries.mockImplementation(
      async (hardwareType: string) => recordedWithoutPower(hardwareType),
    );
    mocks.getFanArchiveSeries.mockResolvedValue(ok([]));
    mocks.getAmbientArchiveSeries.mockResolvedValue(
      ok({ sources: ["Room probe"], buckets: [] }),
    );
    mocks.getCoolingTrend.mockResolvedValue(ok([]));
    mocks.getCoolingFanTrend.mockResolvedValue(
      ok({ series: [], archiveHasReadings: false }),
    );
  });

  it("claims nothing while the archive has not landed", () => {
    const store = mountNotices("7d");

    expect(store.get(coolingPowerNoticeAtom)).toBeNull();
    expect(store.get(coolingFanNoticeAtom)).toBeNull();
    expect(store.get(coolingAmbientSourcesAtom)).toEqual([]);
  });

  it("explains an archive window that recorded no power with Core's support fact", async () => {
    const store = mountNotices("7d");
    await flush();

    // Supported hardware that recorded nothing: not collected, not unsupported.
    expect(store.get(coolingPowerNoticeAtom)).toBe("notCollected");
    // Fan hardware Core reports unsupported, in a window that recorded.
    expect(store.get(coolingFanNoticeAtom)).toBe("unsupported");
  });

  it("claims nothing when the archive read failed", async () => {
    mocks.getDataArchiveSeries.mockResolvedValue({
      status: "error",
      error: "db locked",
    });
    const store = mountNotices("7d");
    await flush();

    expect(store.get(coolingPowerNoticeAtom)).toBeNull();
    expect(store.get(coolingFanNoticeAtom)).toBeNull();
    expect(store.get(coolingAmbientSourcesAtom)).toEqual([]);
  });

  it("reads the daily rollup, not the archive, on the 90d route", async () => {
    mocks.getCoolingTrend.mockResolvedValue(ok([]));
    const store = mountNotices("90d");
    await flush();

    expect(mocks.getDataArchiveSeries).not.toHaveBeenCalled();
    // An empty rollup is not evidence of anything.
    expect(store.get(coolingPowerNoticeAtom)).toBeNull();
    expect(store.get(coolingAmbientSourcesAtom)).toEqual([]);
  });
});
