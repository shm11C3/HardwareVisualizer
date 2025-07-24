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
  const [selectedDataType, setSelectedDataType] = useState<"cpu" | "memory">("cpu");

  useEffect(() => {
    const fetchData = async () => {
      const hardwareType = selectedDataType === "memory" ? "ram" : "cpu";
      const result = await getArchivedRecord(
        hardwareType,
        new Date(period.start),
        new Date(period.end),
      );

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
    const diff =
      new Date(period.end).getTime() - new Date(period.start).getTime();
    return diff > 0 ? Math.max(diff / BUCKET_COUNT, 60000) : 60000; // 最小1分間隔
  }, [period]);

  const bucketedData = useMemo(() => {
    return archivedData.reduce(
      (acc, record) => {
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
  }, [archivedData, step]);

  const { filledLabels, filledChartData } = useMemo(() => {
    const filledChartData: Array<number | null> = [];
    const filledLabels: string[] = [];

    const startAt = new Date(period.start);
    const endAt = new Date(period.end);
    const startBucket = Math.floor(startAt.getTime() / step) * step;
    const endBucket = Math.floor(endAt.getTime() / step) * step;

    for (let t = startBucket; t <= endBucket; t += step) {
      const bucketTime = new Date(t);
      const timeLabel = bucketTime.toLocaleTimeString("ja-JP", {
        hour: "2-digit",
        minute: "2-digit",
      });

      if (!bucketedData[t] || bucketedData[t].length === 0) {
        filledChartData.push(null);
        filledLabels.push(timeLabel);
        continue;
      }

      const aggregatedValue =
        bucketedData[t].reduce((sum, v) => sum + v, 0) / bucketedData[t].length;
      filledChartData.push(Math.round(aggregatedValue * 100) / 100); // 小数点2桁で丸める
      filledLabels.push(timeLabel);
    }

    return { filledLabels, filledChartData };
  }, [bucketedData, step, period]);

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
