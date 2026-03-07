import { useSetAtom } from "jotai";
import { useEffect } from "react";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  cpuUsageHistoryAtom,
  gpuUsageSourceAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import { events } from "@/rspc/bindings";

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

  useEffect(() => {
    const unlisten = events.hardwareMonitorUpdate.listen((event) => {
      const { cpuUsage, memoryUsage, gpuUsage, gpuSource, processorsUsage } =
        event.payload;

      const pad = (arr: number[]) => {
        const padded = Array(
          Math.max(chartConfig.historyLengthSec - arr.length, 0),
        )
          .fill(null)
          .concat(arr);
        return padded.slice(-chartConfig.historyLengthSec);
      };

      setCpuHistory((prev) => {
        const next = [...prev, cpuUsage];
        return pad(next);
      });

      setMemoryHistory((prev) => {
        const next = [...prev, memoryUsage];
        return pad(next);
      });

      if (gpuUsage != null) {
        setGpuHistory((prev) => {
          const next = [...prev, gpuUsage];
          return pad(next);
        });
      }

      if (gpuSource != null) {
        setGpuUsageSource(gpuSource);
      }

      setProcessorsHistory((prev) => {
        const next = [...prev, processorsUsage];
        return next.slice(-chartConfig.historyLengthSec);
      });
    });

    return () => {
      unlisten.then((off) => off());
    };
  }, [
    setCpuHistory,
    setMemoryHistory,
    setGpuHistory,
    setProcessorsHistory,
    setGpuUsageSource,
  ]);
};
