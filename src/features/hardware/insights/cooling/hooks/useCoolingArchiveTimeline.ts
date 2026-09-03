import { useCallback, useEffect, useRef, useState } from "react";
import { chartConfig } from "@/features/hardware/consts/chart";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import {
  type AmbientArchiveSeries,
  type ArchiveSeriesPoint,
  commands,
  type FanArchiveSeries,
} from "@/rspc/bindings";
import { isError } from "@/types/result";
import type { CoolingArchivePeriod } from "../utils/coolingPeriodRoute";
import type { ArchiveTimelineSeries } from "../utils/thermalTimeline";

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

const EMPTY_SERIES: ArchiveTimelineSeries = {
  temperatureAvg: [],
  temperatureMax: [],
  temperatureMin: [],
  cpuUsage: [],
  powerAvg: [],
  powerMax: [],
  powerMin: [],
};

/**
 * Fetch the archive series the 24h/7d/30d timeline lanes are built from -
 * CPU temperature avg/max/min, CPU usage, CPU package power avg/max/min,
 * every archived fan's RPM, and the ambient temperature with its paired
 * Thermal Delta - over one shared time range and bucket width, so every
 * series lands on the same bucket grid.
 *
 * The power, fan and ambient series are requested unconditionally: the
 * archive answers with empty buckets (or, for fans and ambient, no series
 * at all) on a machine that never recorded them, which is exactly the
 * signal those lanes' capability gates read. Asking first would need a
 * second round trip to learn the same thing.
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
   * separate from `hasError` so a fan failure degrades to an unmounted
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
  hasLoaded: boolean;
  hasError: boolean;
};

export const useCoolingArchiveTimeline = (
  minutes: CoolingArchivePeriod | null,
): CoolingArchiveTimeline => {
  const [series, setSeries] = useState<ArchiveTimelineSeries>(EMPTY_SERIES);
  const [fanSeries, setFanSeries] = useState<FanArchiveSeries[]>([]);
  const [fanHasError, setFanHasError] = useState(false);
  const [ambientSeries, setAmbientSeries] =
    useState<AmbientArchiveSeries | null>(null);
  const [ambientHasError, setAmbientHasError] = useState(false);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [hasError, setHasError] = useState(false);
  const { error } = useTauriDialog();
  const requestIdRef = useRef(0);
  // The dialog fires once per failure streak, not once per refresh tick:
  // the interval below reruns every archive-write interval, and a machine
  // with a persistent failure must not stack a dialog per minute.
  const hasReportedErrorRef = useRef(false);

  const stepMs =
    minutes == null
      ? 0
      : STEP_MULTIPLIER[minutes] * chartConfig.archiveUpdateIntervalMilSec;

  const fetchSeries = useCallback(async (): Promise<{
    series: ArchiveTimelineSeries;
    fanSeries: FanArchiveSeries[];
    fanHasError: boolean;
    ambientSeries: AmbientArchiveSeries | null;
    ambientHasError: boolean;
  }> => {
    if (minutes == null) {
      return {
        series: EMPTY_SERIES,
        fanSeries: [],
        fanHasError: false,
        ambientSeries: null,
        ambientHasError: false,
      };
    }

    const endAt = new Date(
      Date.now() - chartConfig.archiveUpdateIntervalMilSec,
    );
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
    // row-per-fan, so how many series exist is not known until they
    // arrive.
    //
    // Resolved rather than rejected on failure, and deliberately not part
    // of the `Promise.all` below: the fan lane is one compact lane among
    // four, and letting its failure reject the batch would throw away the
    // temperature, load and power series that did arrive and turn the
    // whole timeline into a load error. A failed fan read degrades to
    // "capability unknown" - the lane simply does not mount, and the
    // sensor-status note claims nothing (DP-02).
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
          throw new Error(
            `Failed to fetch archived fan series: ${result.error}`,
          );
        }
        return { fanSeries: result.data, fanHasError: false };
      } catch (e) {
        // Logged rather than raised to a dialog: a modal about a secondary
        // lane that has simply not mounted would be louder than the fact
        // it reports.
        console.error(e);
        return { fanSeries: [], fanHasError: true };
      }
    };

    // Kept out of the `Promise.all` for the same reason the fan read is: a
    // failed ambient read must degrade to "capability unknown" - the lane
    // does not mount and the data-state panel names no source - rather
    // than throwing away the lanes that did arrive (DP-02).
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
    };
  }, [minutes, stepMs]);

  useEffect(() => {
    if (minutes == null) {
      requestIdRef.current += 1;
      setSeries(EMPTY_SERIES);
      setFanSeries([]);
      setFanHasError(false);
      setAmbientSeries(null);
      setAmbientHasError(false);
      setHasLoaded(false);
      setHasError(false);
      hasReportedErrorRef.current = false;
      return;
    }

    // A new period starts a fresh load: consumers show a loading state
    // instead of mislabeling the not-yet-fetched window as absent data.
    setHasLoaded(false);
    setHasError(false);
    setFanHasError(false);
    setAmbientHasError(false);
    hasReportedErrorRef.current = false;

    const run = () => {
      const requestId = requestIdRef.current + 1;
      requestIdRef.current = requestId;

      void fetchSeries()
        .then((next) => {
          if (requestIdRef.current === requestId) {
            setSeries(next.series);
            setFanSeries(next.fanSeries);
            setFanHasError(next.fanHasError);
            setAmbientSeries(next.ambientSeries);
            setAmbientHasError(next.ambientHasError);
            setHasLoaded(true);
            setHasError(false);
            hasReportedErrorRef.current = false;
          }
        })
        .catch((e) => {
          console.error(e);
          // A stale request must neither flip the state nor open a dialog.
          if (requestIdRef.current === requestId) {
            setSeries(EMPTY_SERIES);
            setFanSeries([]);
            setFanHasError(false);
            setAmbientSeries(null);
            setAmbientHasError(false);
            setHasLoaded(true);
            setHasError(true);
            if (!hasReportedErrorRef.current) {
              hasReportedErrorRef.current = true;
              void error(String(e));
            }
          }
        });
    };

    run();
    const intervalId = window.setInterval(
      run,
      chartConfig.archiveUpdateIntervalMilSec,
    );

    return () => {
      clearInterval(intervalId);
      // Unmounting invalidates the in-flight request (see the guard above).
      requestIdRef.current += 1;
    };
  }, [fetchSeries, minutes, error]);

  return {
    series,
    fanSeries,
    fanHasError,
    ambientSeries,
    ambientHasError,
    stepMs,
    hasLoaded,
    hasError,
  };
};
