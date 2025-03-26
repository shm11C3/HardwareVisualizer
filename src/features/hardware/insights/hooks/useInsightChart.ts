import {
  type archivePeriods,
  chartConfig,
} from "@/features/hardware/consts/chart";
import type {
  DataArchive,
  GpuDataArchive,
  SingleDataArchive,
} from "@/features/hardware/types/chart";
import type {
  DataStats,
  HardwareDataType,
} from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { sqlitePromise } from "@/lib/sqlite";
import type { HardwareType } from "@/rspc/bindings";
import { useCallback, useEffect, useMemo, useState } from "react";

// 各タイプに合わせた集計関数の定義
const aggregatorMap: Record<DataStats, (values: number[]) => number> = {
  avg: (vals) => vals.reduce((sum, v) => sum + v, 0) / vals.length,
  max: (vals) => Math.max(...vals),
  min: (vals) => Math.min(...vals),
};

const getDataArchiveKey = (
  hardwareType: Exclude<HardwareType, "gpu">,
  dataStats: DataStats,
): keyof DataArchive => {
  const keyMap: Record<
    Exclude<HardwareType, "gpu">,
    Record<string, keyof DataArchive>
  > = {
    cpu: {
      avg: "cpu_avg",
      max: "cpu_max",
      min: "cpu_min",
    },
    memory: {
      avg: "ram_avg",
      max: "ram_max",
      min: "ram_min",
    },
  };

  return keyMap[hardwareType][dataStats];
};

const getGpuDataArchiveKey = (
  dataType: Exclude<HardwareDataType, "clock">,
  dataStats: DataStats,
): keyof GpuDataArchive => {
  const keyMap: Record<
    Exclude<HardwareDataType, "clock">,
    Record<string, keyof GpuDataArchive>
  > = {
    usage: {
      avg: "usage_avg",
      max: "usage_max",
      min: "usage_min",
    },
    temp: {
      avg: "temperature_avg",
      max: "temperature_max",
      min: "temperature_min",
    },
  };

  return keyMap[dataType][dataStats];
};

type UseInsightChartGpuProps = {
  hardwareType: Extract<HardwareType, "gpu">;
  dataStats: DataStats;
  dataType: Exclude<HardwareDataType, "clock">;
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

  const gpuName =
    hardwareType === "gpu" ? (props as UseInsightChartGpuProps).gpuName : "";
  const dataType =
    hardwareType === "gpu"
      ? (props as UseInsightChartGpuProps).dataType
      : undefined;

  const [data, setData] = useState<Array<SingleDataArchive>>([]);

  const step =
    {
      10: 1,
      30: 1,
      60: 1,
      180: 1,
      720: 10,
      1440: 30,
      10080: 60,
      20160: 180,
      43200: 720,
    }[period] * chartConfig.archiveUpdateIntervalMilSec;

  const endAt = useMemo(() => {
    return new Date(Date.now() - offset * step);
  }, [offset, step]);

  const getData = useCallback(async (): Promise<SingleDataArchive[]> => {
    const adjustedEndAt = new Date(
      endAt.getTime() - chartConfig.archiveUpdateIntervalMilSec,
    );
    const startTime = new Date(adjustedEndAt.getTime() - period * 60 * 1000);

    const sql =
      hardwareType === "gpu"
        ? `SELECT ${getGpuDataArchiveKey(
            // biome-ignore lint/style/noNonNullAssertion: <explanation>
            dataType!,
            dataStats,
          )} as value, timestamp
              FROM GPU_DATA_ARCHIVE
              WHERE gpu_name = '${gpuName}'
                AND timestamp BETWEEN '${startTime.toISOString()}'
                AND '${adjustedEndAt.toISOString()}'`
        : `SELECT ${getDataArchiveKey(hardwareType, dataStats)} as value, timestamp
              FROM DATA_ARCHIVE
              WHERE timestamp BETWEEN '${startTime.toISOString()}'
                AND '${adjustedEndAt.toISOString()}'`;

    return await (await sqlitePromise).load(sql);
  }, [endAt, hardwareType, period, dataStats, gpuName, dataType]);

  const formatTemperature = useCallback(
    (value: number) => {
      return settings.temperatureUnit === "F" && dataType === "temp"
        ? (value * 9) / 5 + 32
        : Number.parseFloat(value.toFixed(1));
    },
    [settings.temperatureUnit, dataType],
  );

  useEffect(() => {
    const updateData = () => {
      getData().then((data) =>
        setData(data.map((V) => ({ ...V, value: formatTemperature(V.value) }))),
      );
    };

    updateData();

    const intervalId = setInterval(
      updateData,
      chartConfig.archiveUpdateIntervalMilSec,
    );
    return () => clearInterval(intervalId);
  }, [getData, formatTemperature]);

  const startTime = useMemo(
    () => new Date(endAt.getTime() - period * 60 * 1000),
    [endAt, period],
  );

  const startBucket =
    Math.ceil(
      (startTime.getTime() - chartConfig.archiveUpdateIntervalMilSec) / step,
    ) * step;
  const endBucket =
    Math.ceil(
      (endAt.getTime() - chartConfig.archiveUpdateIntervalMilSec) / step,
    ) * step;

  const bucketedData = useMemo(() => {
    return data.reduce(
      (acc, record) => {
        const recordTime = new Date(record.timestamp).getTime();
        const bucketTimestamp = Math.ceil(recordTime / step) * step;
        if (!acc[bucketTimestamp]) {
          acc[bucketTimestamp] = [];
        }
        acc[bucketTimestamp].push(record.value);
        return acc;
      },
      {} as Record<number, number[]>,
    );
  }, [data, step]);

  const aggregateFn = aggregatorMap[dataStats];

  const dateFormatter = useMemo(() => {
    // 表示オプションを定義（条件に応じてプロパティを設定）
    const dateTimeFormatOptions: Intl.DateTimeFormatOptions = (() => {
      const options: Intl.DateTimeFormatOptions = {};

      // 1440 分以上の場合は年を表示
      if (period >= 1440) {
        options.year = "numeric";
      }
      // 180 分以上の場合は月と日を表示
      if (period >= 180) {
        options.month = "numeric";
        options.day = "2-digit";
      }
      // 10080 分未満の場合は時刻（時間と分）を表示
      if (period < 10080) {
        options.hour = "2-digit";
        options.minute = "2-digit";
      }
      return options;
    })();

    // Intl.DateTimeFormat のインスタンスを生成してキャッシュ
    return new Intl.DateTimeFormat(undefined, dateTimeFormatOptions);
  }, [period]);

  // startBucket～endBucketまで、step刻みでループし、各バケツ内のデータを集計
  const { filledLabels, filledChartData } = useMemo(() => {
    const filledChartData: Array<number | null> = [];
    const filledLabels: string[] = [];

    for (let t = startBucket; t <= endBucket; t += step) {
      if (!bucketedData[t] || bucketedData[t].length <= 0) {
        if (t <= endAt.getTime() - chartConfig.archiveUpdateIntervalMilSec) {
          filledChartData.push(null);
          filledLabels.push(dateFormatter.format(new Date(t)));
        }

        continue;
      }

      const aggregatedValue = aggregateFn(bucketedData[t]);
      filledChartData.push(aggregatedValue);
      filledLabels.push(dateFormatter.format(new Date(t)));
    }

    return { filledLabels, filledChartData };
  }, [
    aggregateFn,
    bucketedData,
    endBucket,
    endAt,
    startBucket,
    step,
    dateFormatter,
  ]);

  return { labels: filledLabels, chartData: filledChartData };
};
