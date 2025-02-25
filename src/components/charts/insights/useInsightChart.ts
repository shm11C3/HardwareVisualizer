import { type archivePeriods, chartConfig } from "@/consts";
import { sqlitePromise } from "@/lib/sqlite";
import type { DataArchive, ShowDataType } from "@/types/chart";
import { useEffect, useMemo, useState } from "react";

// 各タイプに合わせた集計関数の定義
// ※cpu_avg, ram_avgは平均、cpu_max, ram_maxは最大、cpu_min, ram_minは最小を採用する例
const aggregatorMap: Record<ShowDataType, (values: number[]) => number> = {
  cpu_avg: (vals) => vals.reduce((sum, v) => sum + v, 0) / vals.length,
  cpu_max: (vals) => Math.max(...vals),
  cpu_min: (vals) => Math.min(...vals),
  ram_avg: (vals) => vals.reduce((sum, v) => sum + v, 0) / vals.length,
  ram_max: (vals) => Math.max(...vals),
  ram_min: (vals) => Math.min(...vals),
};

const getData = async ({
  endAt,
  period,
}: { endAt: Date; period: (typeof archivePeriods)[number] }): Promise<
  Array<DataArchive>
> => {
  const adjustedEndAt = new Date(
    endAt.getTime() - chartConfig.archiveUpdateIntervalMilSec,
  );
  const sql: string = `SELECT * FROM DATA_ARCHIVE WHERE timestamp BETWEEN '${new Date(adjustedEndAt.getTime() - period * 60 * 1000).toISOString()}' AND '${adjustedEndAt.toISOString()}'`;

  return await (await sqlitePromise).load(sql);
};

export const useInsightChart = ({
  type,
  period,
}: { type: ShowDataType; period: (typeof archivePeriods)[number] }) => {
  const [data, setData] = useState<Array<DataArchive>>([]);
  const [endAt, setEndAt] = useState(new Date());

  useEffect(() => {
    const updateData = () => {
      const newEndAt = new Date();

      setEndAt(newEndAt);
      getData({ endAt: newEndAt, period }).then((data) => setData(data));
    };

    updateData();

    const intervalId = setInterval(
      updateData,
      chartConfig.archiveUpdateIntervalMilSec,
    );
    return () => clearInterval(intervalId);
  }, [period]);

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

  const startTime = new Date(endAt.getTime() - period * 60 * 1000);

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
        acc[bucketTimestamp].push(record[type]);
        return acc;
      },
      {} as Record<number, number[]>,
    );
  }, [data, step, type]);

  const aggregateFn = aggregatorMap[type];

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
