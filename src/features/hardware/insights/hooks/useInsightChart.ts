import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  type archivePeriods,
  chartConfig,
} from "@/features/hardware/consts/chart";
import type { SingleDataArchive } from "@/features/hardware/types/chart";
import type {
  DataStats,
  GpuDataType,
} from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { commands, type HardwareType } from "@/rspc/bindings";
import { isError } from "@/types/result";

// Aggregation function definitions for each type
const aggregatorMap: Record<DataStats, (values: number[]) => number> = {
  avg: (vals) => vals.reduce((sum, v) => sum + v, 0) / vals.length,
  max: (vals) => Math.max(...vals),
  min: (vals) => Math.min(...vals),
};

type UseInsightChartGpuProps = {
  hardwareType: Extract<HardwareType, "gpu">;
  dataStats: DataStats;
  dataType: GpuDataType;
  period: (typeof archivePeriods)[number];
  offset: number;
  gpuName: string;
};

type UseInsightChartProps = {
  hardwareType: Exclude<HardwareType, "gpu">;
  dataStats: DataStats;
  period: (typeof archivePeriods)[number];
  offset: number;
};

export const useInsightChart = (
  props: UseInsightChartGpuProps | UseInsightChartProps,
) => {
  const { hardwareType, dataStats, period, offset } = props;
  const { settings } = useSettingsAtom();

  const gpuName = hardwareType === "gpu" ? props.gpuName : "";
  const dataType = hardwareType === "gpu" ? props.dataType : undefined;

  const [data, setData] = useState<Array<SingleDataArchive>>([]);

  const prevOffsetRef = useRef(offset);
  const activeRequestIdRef = useRef(0);
  const scheduledTimeoutIdRef = useRef<number | null>(null);

  const stepMultiplierMap: Record<(typeof archivePeriods)[number], number> = {
    10: 1,
    30: 1,
    60: 1,
    180: 1,
    720: 10,
    1440: 30,
    10080: 60,
    20160: 180,
    43200: 720,
  };

  const step =
    (stepMultiplierMap[period] ?? 1) * chartConfig.archiveUpdateIntervalMilSec;

  const getData = useCallback(async (): Promise<SingleDataArchive[]> => {
    const endAt = new Date(Date.now() - offset * step);
    const adjustedEndAt = new Date(
      endAt.getTime() - chartConfig.archiveUpdateIntervalMilSec,
    );
    const startTime = new Date(adjustedEndAt.getTime() - period * 60 * 1000);

    if (hardwareType === "gpu") {
      if (!dataType) {
        throw new Error("Data type is required for GPU");
      }

      const result = await commands.getGpuArchiveRecords(
        dataType,
        dataStats,
        gpuName,
        startTime.toISOString(),
        adjustedEndAt.toISOString(),
      );
      if (isError(result)) {
        console.error("Failed to fetch archived GPU records:", result.error);
        return [];
      }

      return result.data;
    }

    const result = await commands.getDataArchiveRecords(
      hardwareType,
      dataStats,
      startTime.toISOString(),
      adjustedEndAt.toISOString(),
    );
    if (isError(result)) {
      console.error("Failed to fetch archived hardware records:", result.error);
      return [];
    }

    return result.data;
  }, [hardwareType, period, dataStats, gpuName, dataType, offset, step]);

  const formatValue = useCallback(
    (value: number | null) => {
      if (value == null) {
        return null;
      }

      if (dataType === "temp" && settings.temperatureUnit === "F") {
        return (value * 9) / 5 + 32;
      }

      // KB => GB
      if (dataType === "dedicatedMemory") {
        return Number.parseFloat((value / (1024 * 1024)).toFixed(1));
      }

      return Number.parseFloat(value.toFixed(1));
    },
    [settings.temperatureUnit, dataType],
  );

  useEffect(() => {
    const isOffsetChanged = prevOffsetRef.current !== offset;
    prevOffsetRef.current = offset;

    // Offset is updated every 100ms while pressing arrows in Insights.
    // Debounce DB reads to avoid spamming when scrubbing.
    const debounceMs = isOffsetChanged ? 250 : 0;

    const requestId = activeRequestIdRef.current + 1;
    activeRequestIdRef.current = requestId;

    const run = async () => {
      try {
        const rows = await getData();
        if (activeRequestIdRef.current !== requestId) {
          return;
        }
        setData(rows.map((v) => ({ ...v, value: formatValue(v.value) })));
      } catch (e) {
        console.error(e);
      }
    };

    if (scheduledTimeoutIdRef.current != null) {
      clearTimeout(scheduledTimeoutIdRef.current);
    }

    scheduledTimeoutIdRef.current = window.setTimeout(run, debounceMs);

    return () => {
      if (scheduledTimeoutIdRef.current != null) {
        clearTimeout(scheduledTimeoutIdRef.current);
        scheduledTimeoutIdRef.current = null;
      }
    };
  }, [getData, formatValue, offset]);

  useEffect(() => {
    // Only auto-refresh the "current" window.
    if (offset !== 0) {
      return;
    }

    const intervalId = window.setInterval(() => {
      const requestId = activeRequestIdRef.current + 1;
      activeRequestIdRef.current = requestId;

      void getData()
        .then((rows) => {
          if (activeRequestIdRef.current !== requestId) {
            return;
          }
          setData(rows.map((v) => ({ ...v, value: formatValue(v.value) })));
        })
        .catch((e) => {
          console.error(e);
        });
    }, chartConfig.archiveUpdateIntervalMilSec);

    return () => clearInterval(intervalId);
  }, [getData, formatValue, offset]);

  const endAtForBucket = useMemo(
    () => new Date(Date.now() - offset * step),
    [offset, step],
  );

  const startTime = useMemo(
    () => new Date(endAtForBucket.getTime() - period * 60 * 1000),
    [endAtForBucket, period],
  );

  const startBucket =
    Math.ceil(
      (startTime.getTime() - chartConfig.archiveUpdateIntervalMilSec) / step,
    ) * step;
  const endBucket =
    Math.ceil(
      (endAtForBucket.getTime() - chartConfig.archiveUpdateIntervalMilSec) /
        step,
    ) * step;

  const bucketedData = useMemo(() => {
    return data.reduce(
      (acc, record) => {
        const recordTime = new Date(record.timestamp).getTime();
        const bucketTimestamp = Math.ceil(recordTime / step) * step;
        if (!acc[bucketTimestamp]) {
          acc[bucketTimestamp] = [];
        }
        if (record.value != null) {
          acc[bucketTimestamp].push(record.value);
        }
        return acc;
      },
      {} as Record<number, number[]>,
    );
  }, [data, step]);

  const aggregateFn = aggregatorMap[dataStats];

  const dateFormatter = useMemo(() => {
    // Define display options (set properties based on conditions)
    const dateTimeFormatOptions: Intl.DateTimeFormatOptions = (() => {
      const options: Intl.DateTimeFormatOptions = {};

      // Show year if 1440 minutes or more
      if (period >= 1440) {
        options.year = "numeric";
      }
      // Show month and day if 180 minutes or more
      if (period >= 180) {
        options.month = "numeric";
        options.day = "2-digit";
      }
      // Show time (hours and minutes) if less than 10080 minutes
      if (period < 10080) {
        options.hour = "2-digit";
        options.minute = "2-digit";
      }
      return options;
    })();

    // Create and cache Intl.DateTimeFormat instance
    return new Intl.DateTimeFormat(undefined, dateTimeFormatOptions);
  }, [period]);

  // Loop from startBucket to endBucket by step, aggregating data in each bucket
  const { filledLabels, filledChartData } = useMemo(() => {
    const filledChartData: Array<number | null> = [];
    const filledLabels: string[] = [];

    for (let t = startBucket; t <= endBucket; t += step) {
      const bucketData = bucketedData[t];
      if (!bucketData || bucketData.length <= 0) {
        if (
          t <=
          endAtForBucket.getTime() - chartConfig.archiveUpdateIntervalMilSec
        ) {
          filledChartData.push(null);
          filledLabels.push(dateFormatter.format(new Date(t)));
        }

        continue;
      }

      const aggregatedValue = aggregateFn(bucketData);
      filledChartData.push(aggregatedValue);
      filledLabels.push(dateFormatter.format(new Date(t)));
    }

    return { filledLabels, filledChartData };
  }, [
    aggregateFn,
    bucketedData,
    endBucket,
    endAtForBucket,
    startBucket,
    step,
    dateFormatter,
  ]);

  const hasData = filledChartData.some((v) => v != null);

  return { labels: filledLabels, chartData: filledChartData, hasData };
};
