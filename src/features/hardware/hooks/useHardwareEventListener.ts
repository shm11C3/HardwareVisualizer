import { useSetAtom } from "jotai";
import { useEffect } from "react";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  cpuTempAtom,
  cpuUsageHistoryAtom,
  gpuDedicatedMemoryKbMapAtom,
  gpuFanSpeedMapAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  gpuUsageSourcesAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
  selectedGpuIdAtom,
  sensorTempsAtom,
} from "@/features/hardware/store/chart";
import { events } from "@/rspc/bindings";

const padHistory = (arr: (number | null)[]): (number | null)[] => {
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
  const setGpuHistories = useSetAtom(gpuUsageHistoriesAtom);
  const setProcessorsHistory = useSetAtom(processorsUsageHistoryAtom);
  const setGpuTempMap = useSetAtom(gpuTempMapAtom);
  const setGpuSources = useSetAtom(gpuUsageSourcesAtom);
  const setGpuMemoryMap = useSetAtom(gpuDedicatedMemoryKbMapAtom);
  const setGpuFanSpeedMap = useSetAtom(gpuFanSpeedMapAtom);
  const setSelectedGpuId = useSetAtom(selectedGpuIdAtom);
  const setCpuTemp = useSetAtom(cpuTempAtom);
  const setSensorTemps = useSetAtom(sensorTempsAtom);

  useEffect(() => {
    const handleHardwareUpdate = (event: {
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
        cpuTemperature: number | null;
        sensorTemperatures: { name: string; value: number }[];
      };
    }) => {
      if (document.hidden) {
        return;
      }

      const {
        cpuUsage,
        memoryUsage,
        gpus,
        processorsUsage,
        cpuTemperature,
        sensorTemperatures,
      } = event.payload;

      setCpuHistory((prev) => padHistory([...prev, cpuUsage]));
      setMemoryHistory((prev) => padHistory([...prev, memoryUsage]));

      // CPU temperature (Windows thermal zones; null where unsupported)
      setCpuTemp(
        cpuTemperature != null ? [{ name: "CPU", value: cpuTemperature }] : [],
      );

      // All named temperature sensors (thermal zones)
      setSensorTemps(sensorTemperatures);

      // Per-GPU usage histories
      setGpuHistories((prev) =>
        gpus.reduce(
          (acc, gpu) => {
            if (gpu.gpuUsage != null) {
              acc[gpu.gpuId] = padHistory([
                ...(acc[gpu.gpuId] ?? []),
                gpu.gpuUsage,
              ]);
            }
            return acc;
          },
          { ...prev },
        ),
      );

      // Temperature from all GPUs
      setGpuTempMap(
        Object.fromEntries(
          gpus
            .filter(
              (g): g is typeof g & { gpuTemperature: number } =>
                g.gpuTemperature != null,
            )
            .map((g) => [
              g.gpuId,
              { name: g.gpuName, value: g.gpuTemperature },
            ]),
        ),
      );

      // Usage sources from all GPUs
      setGpuSources(
        Object.fromEntries(gpus.map((gpu) => [gpu.gpuId, gpu.gpuSource])),
      );

      // Dedicated memory from all GPUs (only update when not null)
      setGpuMemoryMap((prev) =>
        gpus.reduce(
          (acc, gpu) => {
            if (gpu.gpuDedicatedMemoryUsageKb != null) {
              acc[gpu.gpuId] = gpu.gpuDedicatedMemoryUsageKb;
            }
            return acc;
          },
          { ...prev },
        ),
      );

      // Fan speed from all GPUs
      setGpuFanSpeedMap(
        Object.fromEntries(
          gpus
            .filter(
              (g): g is typeof g & { gpuCoolerLevel: number } =>
                g.gpuCoolerLevel != null,
            )
            .map((g) => [
              g.gpuId,
              { name: g.gpuName, value: g.gpuCoolerLevel },
            ]),
        ),
      );

      // Auto-select first GPU if none selected
      setSelectedGpuId((prev) =>
        prev != null ? prev : gpus.length > 0 ? gpus[0].gpuId : null,
      );

      setProcessorsHistory((prev) => {
        const next = [...prev, processorsUsage];
        return next.slice(-chartConfig.historyLengthSec);
      });
    };

    const unlisten = events.hardwareMonitorUpdate.listen(handleHardwareUpdate);

    return () => {
      unlisten.then((off) => off());
    };
  }, [
    setCpuHistory,
    setMemoryHistory,
    setGpuHistories,
    setGpuTempMap,
    setProcessorsHistory,
    setGpuSources,
    setGpuMemoryMap,
    setGpuFanSpeedMap,
    setSelectedGpuId,
    setCpuTemp,
    setSensorTemps,
  ]);
};
