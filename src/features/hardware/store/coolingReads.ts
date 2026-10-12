import { atom } from "jotai";
import { chartConfig } from "@/consts/chart";
import type { CoolingArchivePeriod } from "@/features/hardware/insights/cooling/utils/coolingPeriodRoute";
import type { ArchiveTimelineSeries } from "@/features/hardware/insights/cooling/utils/thermalTimeline";
import {
  coolingArchiveMinutesAtom,
  coolingDailyTrendDaysAtom,
} from "@/features/hardware/store/coolingPeriod";
import { createSharedRead } from "@/features/hardware/store/sharedRead";
import {
  type AmbientArchiveSeries,
  type ArchiveSeriesPoint,
  type CoolingBaselineDelta,
  type CoolingDailyTrendPoint,
  type CoolingFanTrend,
  commands,
  type FanArchiveSeries,
} from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * The Cooling reads that more than one panel renders: each one is a shared
 * read (see `createSharedRead` for the lifetime they all follow). A read a
 * single panel owns stays a hook in that panel.
 *
 * Writing any of these atoms with `"refresh"` refetches it; the panels reach
 * that through the retry hooks and the archive's periodic refresh hook.
 */

/** The idle-baseline delta: the observation strip and the timeline's band. */
export const coolingBaselineDeltaAtom = createSharedRead<
  "current",
  CoolingBaselineDelta
>({
  // The baseline is a Core-owned fact about the current state, not a range
  // the period selector drives, so it has no keyed input.
  params: atom<"current">("current"),
  fetch: async () => {
    const result = await commands.getCoolingBaselineDelta();
    if (isError(result)) {
      throw new Error(
        `Failed to fetch cooling baseline delta: ${result.error}`,
      );
    }
    return result.data;
  },
});

/**
 * The daily cooling rollup for the 90d/1y routes: the timeline, the
 * coverage strip and the sensor notices read it. `[]` is a real empty
 * window; a failed fetch is `failed`, never an empty period.
 */
export const coolingDailyTrendAtom = createSharedRead<
  90 | 365,
  CoolingDailyTrendPoint[]
>({
  params: coolingDailyTrendDaysAtom,
  fetch: async (days) => {
    const result = await commands.getCoolingTrend(days);
    if (isError(result)) {
      throw new Error(`Failed to fetch cooling trend: ${result.error}`);
    }
    return result.data;
  },
});

/**
 * The daily fan rollup for the 90d/1y routes. A separate Core query from
 * the CPU trend: a machine with no readable fan legitimately answers with
 * nothing while the CPU trend still has every day.
 */
export const coolingFanTrendAtom = createSharedRead<90 | 365, CoolingFanTrend>({
  params: coolingDailyTrendDaysAtom,
  fetch: async (days) => {
    const result = await commands.getCoolingFanTrend(days);
    if (isError(result)) {
      throw new Error(`Failed to fetch cooling fan trend: ${result.error}`);
    }
    return result.data;
  },
});

/**
 * Bucket width per period, in multiples of the archive write interval. Same
 * table `useInsightChart` uses, restricted to the periods the Cooling tab
 * routes to the archive (24h/7d/30d).
 */
const STEP_MULTIPLIER: Record<CoolingArchivePeriod, number> = {
  1440: 30,
  10080: 60,
  43200: 720,
};

/**
 * The archive series the 24h/7d/30d timeline lanes are built from - CPU
 * temperature avg/max/min, CPU usage, CPU package power avg/max/min, every
 * archived fan's RPM, and the ambient temperature with its paired Thermal
 * Delta - over one shared time range and bucket width, so every series
 * lands on the same bucket grid.
 *
 * Unlike `useInsightChart` this always reads the current window: the decided
 * layout scrubs history with the single period selector rather than
 * per-chart offset paging.
 */
export type CoolingArchiveTimeline = {
  series: ArchiveTimelineSeries;
  /**
   * One entry per fan the archive holds for this window. Empty on a
   * machine with no readable fan - the fan lane's capability gate.
   */
  fanSeries: FanArchiveSeries[];
  /**
   * The fan read failed on its own while the series above arrived. Kept
   * separate from a failed read so a fan failure degrades to an unmounted
   * lane rather than a whole-timeline load error.
   */
  fanHasError: boolean;
  /**
   * The window's ambient temperature and paired Thermal Delta buckets, or
   * null when the read failed. Empty buckets on a machine with no
   * environmental sensor - the ambient lane's capability gate.
   */
  ambientSeries: AmbientArchiveSeries | null;
  /** The ambient read failed on its own, same contract as `fanHasError`. */
  ambientHasError: boolean;
  stepMs: number;
};

/**
 * The power, fan and ambient series are requested unconditionally: the
 * archive answers with empty buckets (or, for fans and ambient, no series
 * at all) on a machine that never recorded them, which is exactly the
 * signal those lanes' capability gates read. Asking first would need a
 * second round trip to learn the same thing.
 */
const fetchCoolingArchiveTimeline = async (
  minutes: CoolingArchivePeriod,
): Promise<CoolingArchiveTimeline> => {
  const stepMs =
    STEP_MULTIPLIER[minutes] * chartConfig.archiveUpdateIntervalMilSec;
  const endAt = new Date(Date.now() - chartConfig.archiveUpdateIntervalMilSec);
  const startAt = new Date(endAt.getTime() - minutes * 60 * 1000);

  const read = async (
    hardwareType: "cpuTemperature" | "cpu" | "cpuPower",
    stats: "avg" | "max" | "min",
  ): Promise<ArchiveSeriesPoint[]> => {
    const result = await commands.getDataArchiveSeries(
      hardwareType,
      stats,
      startAt.toISOString(),
      endAt.toISOString(),
      stepMs,
      "end",
    );
    if (isError(result)) {
      throw new Error(
        `Failed to fetch archived hardware series: ${result.error}`,
      );
    }
    return result.data;
  };

  // One call for every fan rather than one per fan: the archive is
  // row-per-fan, so how many series exist is not known until they arrive.
  //
  // Resolved rather than rejected on failure, and deliberately not part of
  // the `Promise.all` below: the fan lane is one compact lane among four,
  // and letting its failure reject the batch would throw away the
  // temperature, load and power series that did arrive and turn the whole
  // timeline into a load error. A failed fan read degrades to "capability
  // unknown" - the lane simply does not mount, and the sensor-status note
  // claims nothing (DP-02).
  const readFans = async (): Promise<{
    fanSeries: FanArchiveSeries[];
    fanHasError: boolean;
  }> => {
    try {
      const result = await commands.getFanArchiveSeries(
        startAt.toISOString(),
        endAt.toISOString(),
        stepMs,
        "end",
      );
      if (isError(result)) {
        throw new Error(`Failed to fetch archived fan series: ${result.error}`);
      }
      return { fanSeries: result.data, fanHasError: false };
    } catch (e) {
      // Log only: the secondary lane simply does not mount, and showing a
      // failure for it would be louder than the fact it reports.
      console.error(e);
      return { fanSeries: [], fanHasError: true };
    }
  };

  // Kept out of the `Promise.all` for the same reason the fan read is: a
  // failed ambient read must degrade to "capability unknown" - the lane
  // does not mount and the data-state panel names no source - rather than
  // throwing away the lanes that did arrive (DP-02).
  const readAmbient = async (): Promise<{
    ambientSeries: AmbientArchiveSeries | null;
    ambientHasError: boolean;
  }> => {
    try {
      const result = await commands.getAmbientArchiveSeries(
        startAt.toISOString(),
        endAt.toISOString(),
        stepMs,
        "end",
      );
      if (isError(result)) {
        throw new Error(
          `Failed to fetch archived ambient series: ${result.error}`,
        );
      }
      return { ambientSeries: result.data, ambientHasError: false };
    } catch (e) {
      console.error(e);
      return { ambientSeries: null, ambientHasError: true };
    }
  };

  const [
    [
      temperatureAvg,
      temperatureMax,
      temperatureMin,
      cpuUsage,
      powerAvg,
      powerMax,
      powerMin,
    ],
    fans,
    ambient,
  ] = await Promise.all([
    Promise.all([
      read("cpuTemperature", "avg"),
      read("cpuTemperature", "max"),
      read("cpuTemperature", "min"),
      read("cpu", "avg"),
      read("cpuPower", "avg"),
      read("cpuPower", "max"),
      read("cpuPower", "min"),
    ]),
    readFans(),
    readAmbient(),
  ]);

  return {
    series: {
      temperatureAvg,
      temperatureMax,
      temperatureMin,
      cpuUsage,
      powerAvg,
      powerMax,
      powerMin,
    },
    ...fans,
    ...ambient,
    stepMs,
  };
};

/** The 24h/7d/30d archive timeline: the lanes and the sensor notices. */
export const coolingArchiveAtom = createSharedRead<
  CoolingArchivePeriod,
  CoolingArchiveTimeline
>({
  params: coolingArchiveMinutesAtom,
  fetch: fetchCoolingArchiveTimeline,
});
