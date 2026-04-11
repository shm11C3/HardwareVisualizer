import { useSetAtom } from "jotai";
import { useCallback, useEffect } from "react";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  cpuUsageHistoryAtom,
  gpuDedicatedMemoryKbAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
  gpuUsageSourceAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import { events } from "@/rspc/bindings";

const padHistory = (arr: (number | null)[]): number[] => {
  const padded = Array(Math.max(chartConfig.historyLengthSec - arr.length, 0))
    .fill(null)
    .concat(arr);
  return padded.slice(-chartConfig.historyLengthSec);
};

/**
 * Listen for hardware monitor update events pushed from the backend.
 * Replaces the 4x useUsageUpdater polling hooks with a single event listener.
 */
export const useHardwareEventListener = () => {
  const setCpuHistory = useSetAtom(cpuUsageHistoryAtom);
  const setMemoryHistory = useSetAtom(memoryUsageHistoryAtom);
  const setGpuHistory = useSetAtom(graphicUsageHistoryAtom);
  const setProcessorsHistory = useSetAtom(processorsUsageHistoryAtom);
  const setGpuTemp = useSetAtom(gpuTempAtom);
  const setGpuUsageSource = useSetAtom(gpuUsageSourceAtom);
  const setGpuDedicatedMemory = useSetAtom(gpuDedicatedMemoryKbAtom);
  const setGpuFanSpeed = useSetAtom(gpuFanSpeedAtom);

  const handleHardwareUpdate = useCallback(
    (event: {
      payload: {
        cpuUsage: number;
        memoryUsage: number;
        gpus: {
          gpuId: string;
          gpuName: string;
          gpuUsage: number | null;
          gpuTemperature: number | null;
          gpuSource: string;
          gpuDedicatedMemoryUsageKb: number | null;
          gpuCoolerLevel: number | null;
        }[];
        processorsUsage: number[];
      };
    }) => {
      const { cpuUsage, memoryUsage, gpus, processorsUsage } = event.payload;

      // TODO: Multi-GPU support (#1299) — for now use the first GPU
      const gpu = gpus[0] ?? null;

      setCpuHistory((prev) => padHistory([...prev, cpuUsage]));
      setMemoryHistory((prev) => padHistory([...prev, memoryUsage]));

      if (gpu?.gpuUsage != null) {
        setGpuHistory((prev) => padHistory([...prev, gpu.gpuUsage as number]));
      }

      if (gpu?.gpuName != null && gpu.gpuTemperature != null) {
        setGpuTemp([{ name: gpu.gpuName, value: gpu.gpuTemperature }]);
      }
      setGpuUsageSource(gpu?.gpuSource ?? null);

      if (gpu?.gpuDedicatedMemoryUsageKb != null) {
        setGpuDedicatedMemory(gpu.gpuDedicatedMemoryUsageKb);
      }

      if (gpu?.gpuName != null && gpu.gpuCoolerLevel != null) {
        setGpuFanSpeed([{ name: gpu.gpuName, value: gpu.gpuCoolerLevel }]);
      }

      setProcessorsHistory((prev) => {
        const next = [...prev, processorsUsage];
        return next.slice(-chartConfig.historyLengthSec);
      });
    },
    [
      setCpuHistory,
      setMemoryHistory,
      setGpuHistory,
      setGpuTemp,
      setProcessorsHistory,
      setGpuUsageSource,
      setGpuDedicatedMemory,
      setGpuFanSpeed,
    ],
  );
  useEffect(() => {
    const unlisten = events.hardwareMonitorUpdate.listen(handleHardwareUpdate);

    return () => {
      unlisten.then((off) => off());
    };
  }, [handleHardwareUpdate]);
};
