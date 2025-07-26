import { useEffect, useMemo, useState } from "react";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import type { SingleDataArchive } from "@/features/hardware/types/chart";
import type { ProcessStat } from "../../types/processStats";
import {
  getArchivedRecord,
  getProcessStatsInPeriod,
} from "../funcs/getArchivedRecord";
import type { SnapshotPeriod, UsageRange } from "../types/snapshotType";

export const useSnapshot = () => {
  const { hardwareInfo } = useHardwareInfoAtom();

  // メモリ総量をMB単位で計算
  const totalMemoryMB = useMemo(() => {
    const memorySize = hardwareInfo.memory?.size;
    if (!memorySize) return 32768; // デフォルト32GB

    const [total, unit] = memorySize.split(" ");
    const totalNum = Number.parseFloat(total);

    if (unit === "GB") {
      return Math.ceil(totalNum * 1024); // GBをMBに変換
    }
    if (unit === "MB") {
      return Math.ceil(totalNum);
    }

    return 32768; // デフォルト32GB
  }, [hardwareInfo.memory]);

  const [period, setPeriod] = useState<SnapshotPeriod>({
    start: new Date(Date.now() - 24 * 60 * 60 * 1000).toISOString(),
    end: new Date().toISOString(),
  });
  const [cpuRange, setCpuRange] = useState<UsageRange>({
    type: "cpu",
    value: [0, 100],
  });
  const [selectedDataType, setSelectedDataType] = useState<"cpu" | "memory">(
    "cpu",
  );
  const [archivedData, setArchivedData] = useState<SingleDataArchive[]>([]);
  const [processData, setProcessData] = useState<ProcessStat[]>([]);
  const [memoryMaxOption, setMemoryMaxOption] = useState<
    "128MB" | "256MB" | "512MB" | "1GB" | "2GB" | "8GB" | "device"
  >("device");

  // 選択されたオプションに基づく実際のメモリ最大値
  const selectedMemoryMaxMB = useMemo(() => {
    switch (memoryMaxOption) {
      case "128MB":
        return 128;
      case "256MB":
        return 256;
      case "512MB":
        return 512;
      case "1GB":
        return 1024;
      case "2GB":
        return 2048;
      case "8GB":
        return 8192;
      default:
        return totalMemoryMB; // デバイスのメモリ総量
    }
  }, [memoryMaxOption, totalMemoryMB]);

  const [memoryRange, setMemoryRange] = useState<UsageRange>({
    type: "memory",
    value: [0, 32768], // デフォルト0MB - 32GB の範囲
  });

  // メモリ最大値選択が変更されたときにmemoryRangeの最大値を更新
  useEffect(() => {
    setMemoryRange((prev) => ({
      ...prev,
      value: [prev.value[0], selectedMemoryMaxMB],
    }));
  }, [selectedMemoryMaxMB]);

  useEffect(() => {
    const fetchData = async () => {
      const startDate = new Date(period.start);
      const endDate = new Date(period.end);

      // ハードウェアデータ取得
      const hardwareType = selectedDataType === "memory" ? "ram" : "cpu";
      const archivedResult = await getArchivedRecord(
        hardwareType,
        startDate,
        endDate,
      );
      setArchivedData(archivedResult);

      // プロセスデータ取得
      const processResult = await getProcessStatsInPeriod(
        startDate,
        endDate,
      );
      setProcessData(processResult);
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

  // プロセスデータの範囲フィルタリング
  const filteredProcessData = useMemo(() => {
    if (!processData || !Array.isArray(processData)) {
      return [];
    }
    return processData.filter((process) => {
      const cpuUsage = process.avg_cpu_usage || 0;
      const memoryUsageKB = process.avg_memory_usage || 0;
      const memoryUsageMB = memoryUsageKB / 1024; // KBからMBに変換

      // CPU使用率の範囲チェック（パーセンテージ）
      const cpuInRange =
        cpuUsage >= cpuRange.value[0] && cpuUsage <= cpuRange.value[1];

      // メモリ使用量の範囲チェック（MB）
      // memoryRange.value[0]とvalue[1]はMB単位での範囲を表す
      const memoryInRange =
        memoryUsageMB >= memoryRange.value[0] &&
        memoryUsageMB <= memoryRange.value[1];

      return cpuInRange && memoryInRange;
    });
  }, [processData, cpuRange, memoryRange]);

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
    processData,
    filteredProcessData,
    totalMemoryMB,
    memoryMaxOption,
    setMemoryMaxOption,
    selectedMemoryMaxMB,
  };
};
