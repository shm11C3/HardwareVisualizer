import {
  cpuFanSpeedAtom,
  cpuTempAtom,
  cpuUsageHistoryAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/atom/chart";
import { chartConfig } from "@/consts/chart";
import { type NameValue, type Result, commands } from "@/rspc/bindings";
import type { ChartDataType } from "@/types/hardwareDataType";
import { isError, isOk, isResult } from "@/types/result";
import { type PrimitiveAtom, useSetAtom } from "jotai";
import { useEffect } from "react";

/**
 * ハードウェア使用率の履歴を更新する
 */
export const useUsageUpdater = (dataType: ChartDataType) => {
  type AtomActionMapping = {
    atom: PrimitiveAtom<number[]>;
    action: () => Promise<number> | Promise<Result<number, string>>;
  };

  const mapping: Record<ChartDataType, AtomActionMapping> = {
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
  };

  const setHistory = useSetAtom(mapping[dataType].atom);
  const getUsage = mapping[dataType].action;

  useEffect(() => {
    const intervalId = setInterval(async () => {
      const result = await getUsage();

      if (isResult(result) && isError(result)) return;
      const usage = isResult(result) ? result.data : result;

      setHistory((prev) => {
        const newHistory = [...prev, usage];

        // 履歴保持数に満たない場合は0で埋める
        const paddedHistory = Array(
          Math.max(chartConfig.historyLengthSec - newHistory.length, 0),
        )
          .fill(null)
          .concat(newHistory);
        return paddedHistory.slice(-chartConfig.historyLengthSec);
      });
    }, 1000);

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
      fetchData;
    }, 10000);

    return () => clearInterval(intervalId);
  }, [setData, getData]);
};
