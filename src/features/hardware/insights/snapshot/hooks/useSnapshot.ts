import { useEffect, useMemo, useState } from "react";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import type { ArchiveSeriesPoint } from "@/rspc/bindings";
import type { ProcessStat } from "../../types/processStats";
import {
  getArchivedRecord,
  getProcessStatsInPeriod,
} from "../funcs/getArchivedRecord";
import type { SnapshotPeriod, UsageRange } from "../types/snapshotType";

export const useSnapshot = () => {
  const { hardwareInfo } = useHardwareInfoAtom();
  const { error } = useTauriDialog();

  // Calculate total memory in MB
  const totalMemoryMB = useMemo(() => {
    const memorySize = hardwareInfo.memory?.size;
    if (!memorySize) return 32768; // Default 32GB

    const [total, unit] = memorySize.split(" ");
    const totalNum = Number.parseFloat(total);

    if (unit === "GB") {
      return Math.ceil(totalNum * 1024); // Convert GB to MB
    }
    if (unit === "MB") {
      return Math.ceil(totalNum);
    }

    return 32768; // Default 32GB
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
  const [archivedData, setArchivedData] = useState<ArchiveSeriesPoint[]>([]);
  const [processData, setProcessData] = useState<ProcessStat[]>([]);
  const [memoryMaxOption, setMemoryMaxOption] = useState<
    "128MB" | "256MB" | "512MB" | "1GB" | "2GB" | "8GB" | "device"
  >("device");

  // Actual memory max based on selected option
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
        return totalMemoryMB; // Device total memory
    }
  }, [memoryMaxOption, totalMemoryMB]);

  const [memoryRange, setMemoryRange] = useState<UsageRange>({
    type: "memory",
    value: [0, 32768], // Default 0MB - 32GB range
  });

  // Update memoryRange max value when memory max selection changes
  useEffect(() => {
    setMemoryRange((prev) => ({
      ...prev,
      value: [prev.value[0], selectedMemoryMaxMB],
    }));
  }, [selectedMemoryMaxMB]);

  /** Maximum number of data points to display in the chart. */
  const BUCKET_COUNT = 100;

  const step = useMemo(() => {
    const startTime = new Date(period.start).getTime();
    const endTime = new Date(period.end).getTime();
    const diff = endTime - startTime;

    if (diff <= 0) return 60000;

    return Math.ceil(Math.max(diff / BUCKET_COUNT, 60000));
  }, [period]);

  useEffect(() => {
    const fetchData = async () => {
      const startDate = new Date(period.start);
      const endDate = new Date(period.end);

      try {
        // Get hardware data
        const hardwareType = selectedDataType === "memory" ? "ram" : "cpu";
        const archivedResult = await getArchivedRecord(
          hardwareType,
          startDate,
          endDate,
          step,
        );
        setArchivedData(archivedResult);

        // Get process data
        const processResult = await getProcessStatsInPeriod(startDate, endDate);
        setProcessData(processResult);
      } catch (err) {
        console.error(err);
        setArchivedData([]);
        setProcessData([]);
        void error(String(err));
      }
    };

    void fetchData();
  }, [period, selectedDataType, step, error]);

  const dateFormatter = useMemo(() => {
    const periodMinutes =
      (new Date(period.end).getTime() - new Date(period.start).getTime()) /
      (1000 * 60);

    // Define display options (same conditions as useInsightChart)
    const dateTimeFormatOptions: Intl.DateTimeFormatOptions = (() => {
      const options: Intl.DateTimeFormatOptions = {};

      // Show year if 1440 minutes or more
      if (periodMinutes >= 1440) {
        options.year = "numeric";
      }
      // Show month and day if 180 minutes or more
      if (periodMinutes >= 180) {
        options.month = "numeric";
        options.day = "2-digit";
      }
      // Show time (hours and minutes) if less than 10080 minutes
      if (periodMinutes < 10080) {
        options.hour = "2-digit";
        options.minute = "2-digit";
      }
      return options;
    })();

    // Create and cache Intl.DateTimeFormat instance
    return new Intl.DateTimeFormat(undefined, dateTimeFormatOptions);
  }, [period]);

  // Range filtering for process data
  const filteredProcessData = useMemo(() => {
    if (!processData || !Array.isArray(processData)) {
      return [];
    }
    return processData.filter((process) => {
      const cpuUsage = process.avg_cpu_usage || 0;
      const memoryUsageKB = process.avg_memory_usage || 0;
      const memoryUsageMB = memoryUsageKB / 1024; // Convert KB to MB

      // CPU usage range check (percentage)
      const cpuInRange =
        cpuUsage >= cpuRange.value[0] && cpuUsage <= cpuRange.value[1];

      // Memory usage range check (MB)
      // memoryRange.value[0] and value[1] represent range in MB
      const memoryInRange =
        memoryUsageMB >= memoryRange.value[0] &&
        memoryUsageMB <= memoryRange.value[1];

      return cpuInRange && memoryInRange;
    });
  }, [processData, cpuRange, memoryRange]);

  const { filledLabels, filledChartData } = useMemo(() => {
    const startAt = new Date(period.start);
    const endAt = new Date(period.end);

    // Return empty data if start time is after end time
    if (startAt.getTime() >= endAt.getTime()) {
      return { filledLabels: [], filledChartData: [] };
    }

    return {
      filledLabels: archivedData.map((point) =>
        dateFormatter.format(new Date(point.timestamp)),
      ),
      filledChartData: archivedData.map((point) =>
        point.value == null ? null : Math.round(point.value * 100) / 100,
      ),
    };
  }, [archivedData, period, dateFormatter]);

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
