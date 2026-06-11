import { useEffect, useRef, useState } from "react";
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
import { useTauriDialog } from "@/hooks/useTauriDialog";
import {
  commands,
  type HardwareType,
  type TemperatureUnit,
} from "@/rspc/bindings";
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

type InsightArchiveRequest = {
  hardwareType: HardwareType;
  dataStats: DataStats;
  period: (typeof archivePeriods)[number];
  offset: number;
  step: number;
  dataType: GpuDataType | undefined;
  gpuName: string;
};

const getInsightArchiveData = async ({
  hardwareType,
  dataStats,
  period,
  offset,
  step,
  dataType,
  gpuName,
}: InsightArchiveRequest): Promise<SingleDataArchive[]> => {
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
      throw new Error(`Failed to fetch archived GPU records: ${result.error}`);
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
    throw new Error(
      `Failed to fetch archived hardware records: ${result.error}`,
    );
  }

  return result.data;
};

const formatInsightValue = (
  value: number | null,
  dataType: GpuDataType | undefined,
  temperatureUnit: TemperatureUnit,
) => {
  if (value == null) {
    return null;
  }

  if (dataType === "temp" && temperatureUnit === "F") {
    return (value * 9) / 5 + 32;
  }

  // KB => GB
  if (dataType === "dedicatedMemory") {
    return Number.parseFloat((value / (1024 * 1024)).toFixed(1));
  }

  return Number.parseFloat(value.toFixed(1));
};

export const useInsightChart = (
  props: UseInsightChartGpuProps | UseInsightChartProps,
) => {
  const { hardwareType, dataStats, period, offset } = props;
  const { settings } = useSettingsAtom();
  const { temperatureUnit } = settings;
  const { error } = useTauriDialog();

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
        const rows = await getInsightArchiveData({
          hardwareType,
          dataStats,
          period,
          offset,
          step,
          dataType,
          gpuName,
        });
        if (activeRequestIdRef.current !== requestId) {
          return;
        }
        setData(
          rows.map((v) => ({
            ...v,
            value: formatInsightValue(v.value, dataType, temperatureUnit),
          })),
        );
      } catch (e) {
        console.error(e);
        if (activeRequestIdRef.current === requestId) {
          setData([]);
        }
        void error(String(e));
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
  }, [
    hardwareType,
    dataStats,
    period,
    offset,
    step,
    dataType,
    gpuName,
    temperatureUnit,
    error,
  ]);

  useEffect(() => {
    // Only auto-refresh the "current" window.
    if (offset !== 0) {
      return;
    }

    const intervalId = window.setInterval(() => {
      const requestId = activeRequestIdRef.current + 1;
      activeRequestIdRef.current = requestId;

      void getInsightArchiveData({
        hardwareType,
        dataStats,
        period,
        offset,
        step,
        dataType,
        gpuName,
      })
        .then((rows) => {
          if (activeRequestIdRef.current !== requestId) {
            return;
          }
          setData(
            rows.map((v) => ({
              ...v,
              value: formatInsightValue(v.value, dataType, temperatureUnit),
            })),
          );
        })
        .catch((e) => {
          console.error(e);
          if (activeRequestIdRef.current === requestId) {
            setData([]);
          }
          void error(String(e));
        });
    }, chartConfig.archiveUpdateIntervalMilSec);

    return () => clearInterval(intervalId);
  }, [
    hardwareType,
    dataStats,
    period,
    offset,
    step,
    dataType,
    gpuName,
    temperatureUnit,
    error,
  ]);

  const endAtForBucket = new Date(Date.now() - offset * step);

  const startTime = new Date(endAtForBucket.getTime() - period * 60 * 1000);

  const startBucket =
    Math.ceil(
      (startTime.getTime() - chartConfig.archiveUpdateIntervalMilSec) / step,
    ) * step;
  const endBucket =
    Math.ceil(
      (endAtForBucket.getTime() - chartConfig.archiveUpdateIntervalMilSec) /
        step,
    ) * step;

  const bucketedData = (() => {
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
  })();

  const aggregateFn = aggregatorMap[dataStats];

  const dateFormatter = (() => {
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
  })();

  // Loop from startBucket to endBucket by step, aggregating data in each bucket
  const { filledLabels, filledChartData } = (() => {
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
  })();

  const hasData = filledChartData.some((v) => v != null);

  return { labels: filledLabels, chartData: filledChartData, hasData };
};
