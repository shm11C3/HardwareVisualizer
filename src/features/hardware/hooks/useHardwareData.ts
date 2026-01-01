import { type PrimitiveAtom, useSetAtom } from "jotai";
import { useEffect } from "react";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  cpuFanSpeedAtom,
  cpuTempAtom,
  cpuUsageHistoryAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import type {
  ChartDataHardwareType,
  ChartDataType,
} from "@/features/hardware/types/hardwareDataType";
import { commands, type NameValue, type Result } from "@/rspc/bindings";
import { isError, isOk, isResult } from "@/types/result";

/**
 * Update hardware usage history
 */
export const useUsageUpdater = (dataType: ChartDataHardwareType) => {
  type AtomActionMapping = {
    atom: PrimitiveAtom<number[]> | PrimitiveAtom<number[][]>;
    action: () =>
      | Promise<number>
      | Promise<Result<number, string>>
      | Promise<number[]>;
  };

  const mapping: Record<ChartDataHardwareType, AtomActionMapping> = {
    cpu: {
      atom: cpuUsageHistoryAtom,
      action: commands.getCpuUsage,
    },
    memory: {
      atom: memoryUsageHistoryAtom,
      action: commands.getMemoryUsage,
    },
    gpu: {
      atom: graphicUsageHistoryAtom,
      action: commands.getGpuUsage,
    },
    processors: {
      atom: processorsUsageHistoryAtom,
      action: commands.getProcessorsUsage,
    },
  };

  const setHistory = useSetAtom(mapping[dataType].atom);
  const getUsage = mapping[dataType].action;

  useEffect(() => {
    const fetchAndUpdate = async () => {
      const result = await getUsage();

      if (isResult(result) && isError(result)) return;
      const usage = isResult(result) ? result.data : result;

      // CPU and GPU usage may be returned as array
      if (Array.isArray(usage)) {
        setHistory((prev: number[][]) => {
          const newHistory = [...prev, usage];
          return newHistory.slice(-chartConfig.historyLengthSec);
        });
        return;
      }

      // Add directly to history if single number
      setHistory((prev: number[]) => {
        const newHistory = [...prev, usage];

        // Pad with 0 if not enough history
        const paddedHistory = Array(
          Math.max(chartConfig.historyLengthSec - newHistory.length, 0),
        )
          .fill(null)
          .concat(newHistory);
        return paddedHistory.slice(-chartConfig.historyLengthSec);
      });
    };

    fetchAndUpdate();
    const intervalId = setInterval(fetchAndUpdate, 1000);

    return () => clearInterval(intervalId);
  }, [setHistory, getUsage]);
};

export const useHardwareUpdater = (
  hardType: Exclude<ChartDataType, "memory">,
  dataType: "temp" | "fan",
) => {
  type AtomActionMapping = {
    atom: PrimitiveAtom<NameValue[]>;
    action: () => Promise<Result<NameValue[], string>>;
  };

  const mapping: Record<
    Exclude<ChartDataType, "memory">,
    Record<"temp" | "fan", AtomActionMapping>
  > = {
    cpu: {
      temp: {
        atom: cpuTempAtom,
        action: () => {
          console.error("Not implemented");
          return Promise.resolve({ status: "error", error: "Not implemented" });
        },
      },
      fan: {
        atom: cpuFanSpeedAtom,
        action: () => {
          console.error("Not implemented");
          return Promise.resolve({ status: "error", error: "Not implemented" });
        },
      },
    },
    gpu: {
      fan: {
        atom: gpuFanSpeedAtom,
        action: commands.getNvidiaGpuCooler,
      },
      temp: {
        atom: gpuTempAtom,
        action: commands.getGpuTemperature,
      },
    },
  };

  const setData = useSetAtom(mapping[hardType][dataType].atom);
  const getData = mapping[hardType][dataType].action;

  useEffect(() => {
    const fetchData = async () => {
      const result = await getData();

      if (isOk(result)) {
        setData(result.data);
      }
    };

    fetchData();

    const intervalId = setInterval(async () => {
      fetchData();
    }, 10000);

    return () => clearInterval(intervalId);
  }, [setData, getData]);
};
