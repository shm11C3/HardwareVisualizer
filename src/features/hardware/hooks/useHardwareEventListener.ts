import { useSetAtom } from "jotai";
import { useCallback, useEffect } from "react";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  cpuUsageHistoryAtom,
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
  const setGpuUsageSource = useSetAtom(gpuUsageSourceAtom);

  const handleHardwareUpdate = useCallback(
    (event: {
      payload: {
        cpuUsage: number;
        memoryUsage: number;
        gpuUsage: number | null;
        gpuSource: string | null;
        processorsUsage: number[];
      };
    }) => {
      const { cpuUsage, memoryUsage, gpuUsage, gpuSource, processorsUsage } =
        event.payload;

      setCpuHistory((prev) => padHistory([...prev, cpuUsage]));
      setMemoryHistory((prev) => padHistory([...prev, memoryUsage]));

      if (gpuUsage != null) {
        setGpuHistory((prev) => padHistory([...prev, gpuUsage]));
      }

      setGpuUsageSource(gpuSource);

      setProcessorsHistory((prev) => {
        const next = [...prev, processorsUsage];
        return next.slice(-chartConfig.historyLengthSec);
      });
    },
    [
      setCpuHistory,
      setMemoryHistory,
      setGpuHistory,
      setProcessorsHistory,
      setGpuUsageSource,
    ],
  );
  useEffect(() => {
    const unlisten = events.hardwareMonitorUpdate.listen(handleHardwareUpdate);

    return () => {
      unlisten.then((off) => off());
    };
  }, [handleHardwareUpdate]);
};
