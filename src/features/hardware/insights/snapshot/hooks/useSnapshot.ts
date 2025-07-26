import { useEffect, useMemo, useState } from "react";
import type { SingleDataArchive } from "@/features/hardware/types/chart";
import { getArchivedRecord } from "../funcs/getArchivedRecord";
import type { SnapshotPeriod, UsageRange } from "../types/snapshotType";

export const useSnapshot = () => {
  const [period, setPeriod] = useState<SnapshotPeriod>({
    start: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
    end: new Date().toISOString(),
  });
  const [cpuRange, setCpuRange] = useState<UsageRange>({
    type: "cpu",
    value: [0, 100],
  });
  const [memoryRange, setMemoryRange] = useState<UsageRange>({
    type: "memory",
    value: [0, 100],
  });
  const [archivedData, setArchivedData] = useState<SingleDataArchive[]>([]);
  const [selectedDataType, setSelectedDataType] = useState<"cpu" | "memory">(
    "cpu",
  );

  useEffect(() => {
    const fetchData = async () => {
      const hardwareType = selectedDataType === "memory" ? "ram" : "cpu";
      const startDate = new Date(period.start);
      const endDate = new Date(period.end);

      const result = await getArchivedRecord(hardwareType, startDate, endDate);

      setArchivedData(result);
    };

    fetchData();
  }, [period, selectedDataType]);

  /**
   * チャートに表示するデータポイントの最大数
   * 期間に関係なく100個のバケットに分割してデータを集約する
   */
  const BUCKET_COUNT = 100;

  const step = useMemo(() => {
    const startTime = new Date(period.start).getTime();
    const endTime = new Date(period.end).getTime();
    const diff = endTime - startTime;

    // 開始時刻が終了時刻より後の場合は無効
    if (diff <= 0) return 60000;

    // ステップを整数にして計算の一貫性を保つ
    return Math.floor(Math.max(diff / BUCKET_COUNT, 60000));
  }, [period]);

  const bucketedData = useMemo(() => {
    const result = archivedData.reduce(
      (acc, record, _index) => {
        if (record.value == null) return acc;

        const recordTime = new Date(record.timestamp).getTime();
        const bucketTimestamp = Math.floor(recordTime / step) * step;

        if (!acc[bucketTimestamp]) {
          acc[bucketTimestamp] = [];
        }
        acc[bucketTimestamp].push(record.value);

        return acc;
      },
      {} as Record<number, number[]>,
    );

    return result;
  }, [archivedData, step]);

  const dateFormatter = useMemo(() => {
    const periodMinutes =
      (new Date(period.end).getTime() - new Date(period.start).getTime()) /
      (1000 * 60);

    // 表示オプションを定義（useInsightChartと同じ条件）
    const dateTimeFormatOptions: Intl.DateTimeFormatOptions = (() => {
      const options: Intl.DateTimeFormatOptions = {};

      // 1440 分以上の場合は年を表示
      if (periodMinutes >= 1440) {
        options.year = "numeric";
      }
      // 180 分以上の場合は月と日を表示
      if (periodMinutes >= 180) {
        options.month = "numeric";
        options.day = "2-digit";
      }
      // 10080 分未満の場合は時刻（時間と分）を表示
      if (periodMinutes < 10080) {
        options.hour = "2-digit";
        options.minute = "2-digit";
      }
      return options;
    })();

    // Intl.DateTimeFormat のインスタンスを生成してキャッシュ
    return new Intl.DateTimeFormat(undefined, dateTimeFormatOptions);
  }, [period]);

  const { filledLabels, filledChartData } = useMemo(() => {
    const filledChartData: Array<number | null> = [];
    const filledLabels: string[] = [];

    const startAt = new Date(period.start);
    const endAt = new Date(period.end);

    // 開始時刻が終了時刻より後の場合は空のデータを返す
    if (startAt.getTime() >= endAt.getTime()) {
      return { filledLabels, filledChartData };
    }

    const startBucket = Math.floor(startAt.getTime() / step) * step;
    const endBucket = Math.floor(endAt.getTime() / step) * step;

    for (let t = startBucket; t <= endBucket; t += step) {
      const bucketTime = new Date(t);
      const timeLabel = dateFormatter.format(bucketTime);

      // 直接一致を先に確認
      if (bucketedData[t] && bucketedData[t].length > 0) {
        const aggregatedValue =
          bucketedData[t].reduce((sum, v) => sum + v, 0) /
          bucketedData[t].length;
        filledChartData.push(Math.round(aggregatedValue * 100) / 100);
        filledLabels.push(timeLabel);
        continue;
      }

      // 近接バケット検索
      let foundData = null;
      const tolerance = step * 0.5;

      for (const bucketKey of Object.keys(bucketedData)) {
        const bucketTimestamp = Number(bucketKey);
        if (
          Math.abs(bucketTimestamp - t) <= tolerance &&
          bucketedData[bucketTimestamp]?.length > 0
        ) {
          const aggregatedValue =
            bucketedData[bucketTimestamp].reduce((sum, v) => sum + v, 0) /
            bucketedData[bucketTimestamp].length;
          foundData = Math.round(aggregatedValue * 100) / 100;
          break;
        }
      }

      filledChartData.push(foundData);
      filledLabels.push(timeLabel);
    }

    return { filledLabels, filledChartData };
  }, [bucketedData, step, period, dateFormatter]);

  return {
    period,
    setPeriod,
    cpuRange,
    setCpuRange,
    memoryRange,
    setMemoryRange,
    selectedDataType,
    setSelectedDataType,
    filledLabels,
    filledChartData,
  };
};
