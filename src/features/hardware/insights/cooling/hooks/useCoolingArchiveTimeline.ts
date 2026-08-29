import { useCallback, useEffect, useRef, useState } from "react";
import { chartConfig } from "@/features/hardware/consts/chart";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type ArchiveSeriesPoint, commands } from "@/rspc/bindings";
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
};

/**
 * Fetch the four archive series the 24h/7d/30d timeline lanes are built
 * from - CPU temperature avg/max/min plus CPU usage - over one shared time
 * range and bucket width, so every series lands on the same bucket grid.
 *
 * Unlike `useInsightChart` this always reads the current window: the decided
 * layout scrubs history with the single period selector rather than
 * per-chart offset paging.
 */
export const useCoolingArchiveTimeline = (
  minutes: CoolingArchivePeriod | null,
) => {
  const [series, setSeries] = useState<ArchiveTimelineSeries>(EMPTY_SERIES);
  const { error } = useTauriDialog();
  const requestIdRef = useRef(0);

  const stepMs =
    minutes == null
      ? 0
      : STEP_MULTIPLIER[minutes] * chartConfig.archiveUpdateIntervalMilSec;

  const fetchSeries = useCallback(async (): Promise<ArchiveTimelineSeries> => {
    if (minutes == null) {
      return EMPTY_SERIES;
    }

    const endAt = new Date(
      Date.now() - chartConfig.archiveUpdateIntervalMilSec,
    );
    const startAt = new Date(endAt.getTime() - minutes * 60 * 1000);

    const read = async (
      hardwareType: "cpuTemperature" | "cpu",
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

    const [temperatureAvg, temperatureMax, temperatureMin, cpuUsage] =
      await Promise.all([
        read("cpuTemperature", "avg"),
        read("cpuTemperature", "max"),
        read("cpuTemperature", "min"),
        read("cpu", "avg"),
      ]);

    return { temperatureAvg, temperatureMax, temperatureMin, cpuUsage };
  }, [minutes, stepMs]);

  useEffect(() => {
    if (minutes == null) {
      requestIdRef.current += 1;
      setSeries(EMPTY_SERIES);
      return;
    }

    const run = () => {
      const requestId = requestIdRef.current + 1;
      requestIdRef.current = requestId;

      void fetchSeries()
        .then((next) => {
          if (requestIdRef.current === requestId) {
            setSeries(next);
          }
        })
        .catch((e) => {
          console.error(e);
          if (requestIdRef.current === requestId) {
            setSeries(EMPTY_SERIES);
          }
          void error(String(e));
        });
    };

    run();
    const intervalId = window.setInterval(
      run,
      chartConfig.archiveUpdateIntervalMilSec,
    );

    return () => clearInterval(intervalId);
  }, [fetchSeries, minutes, error]);

  return { series, stepMs };
};
