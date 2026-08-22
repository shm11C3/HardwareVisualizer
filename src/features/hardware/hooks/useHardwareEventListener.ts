import { useSetAtom } from "jotai";
import { useCallback, useEffect, useRef } from "react";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  asLiveGpuId,
  type LiveGpuId,
  liveGpuRecord,
} from "@/features/hardware/gpuIdentity";
import {
  cpuTempAtom,
  cpuUsageHistoryAtom,
  gpuDedicatedMemoryKbMapAtom,
  gpuFanSpeedMapAtom,
  gpuNamesAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  gpuUsageSourcesAtom,
  memoryUsageHistoryAtom,
  motherboardFanSpeedsAtom,
  motherboardTempsAtom,
  powerDrawAtom,
  powerDrawAvailableAtom,
  processorsUsageHistoryAtom,
  selectedGpuIdAtom,
  sensorTempsAtom,
} from "@/features/hardware/store/chart";
import { events, type HardwareMonitorUpdate } from "@/rspc/bindings";

const padHistory = (arr: (number | null)[]): (number | null)[] => {
  const padded = Array(Math.max(chartConfig.historyLengthSec - arr.length, 0))
    .fill(null)
    .concat(arr);
  return padded.slice(-chartConfig.historyLengthSec);
};

// One omitted sample can be a provider hiccup. Three consecutive visible
// samples establish that the adapter is no longer part of the live set while
// keeping unplug/fallback feedback within a few seconds at the 1 Hz cadence.
const GPU_RETIREMENT_MISSED_SAMPLES = 3;

/**
 * Listen for hardware monitor update events pushed from the backend.
 * Replaces the 4x useUsageUpdater polling hooks with a single event listener.
 */
export const useHardwareEventListener = () => {
  const gpuMissedSamples = useRef(new Map<LiveGpuId, number>());
  const setCpuHistory = useSetAtom(cpuUsageHistoryAtom);
  const setMemoryHistory = useSetAtom(memoryUsageHistoryAtom);
  const setGpuHistories = useSetAtom(gpuUsageHistoriesAtom);
  const setProcessorsHistory = useSetAtom(processorsUsageHistoryAtom);
  const setGpuTempMap = useSetAtom(gpuTempMapAtom);
  const setGpuNames = useSetAtom(gpuNamesAtom);
  const setGpuSources = useSetAtom(gpuUsageSourcesAtom);
  const setGpuMemoryMap = useSetAtom(gpuDedicatedMemoryKbMapAtom);
  const setGpuFanSpeedMap = useSetAtom(gpuFanSpeedMapAtom);
  const setSelectedGpuId = useSetAtom(selectedGpuIdAtom);
  const setCpuTemp = useSetAtom(cpuTempAtom);
  const setSensorTemps = useSetAtom(sensorTempsAtom);
  const setMotherboardTemps = useSetAtom(motherboardTempsAtom);
  const setMotherboardFanSpeeds = useSetAtom(motherboardFanSpeedsAtom);
  const setPowerDrawAvailable = useSetAtom(powerDrawAvailableAtom);
  const setPowerDraw = useSetAtom(powerDrawAtom);

  const handleHardwareUpdate = useCallback(
    (event: { payload: HardwareMonitorUpdate }) => {
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
        motherboardTemperatures,
        motherboardFanSpeeds,
        cpuPowerWatts,
        gpuPowerWatts,
        anePowerWatts,
        packagePowerWatts,
      } = event.payload;

      const currentGpuIds = gpus.map((gpu) => asLiveGpuId(gpu.gpuId));
      const currentGpuIdSet = new Set(currentGpuIds);
      const retiredGpuIds = new Set<LiveGpuId>();

      for (const gpuId of currentGpuIds) {
        gpuMissedSamples.current.set(gpuId, 0);
      }
      for (const [gpuId, missedSamples] of gpuMissedSamples.current) {
        if (currentGpuIdSet.has(gpuId)) {
          continue;
        }

        const nextMissedSamples = missedSamples + 1;
        if (nextMissedSamples >= GPU_RETIREMENT_MISSED_SAMPLES) {
          retiredGpuIds.add(gpuId);
          gpuMissedSamples.current.delete(gpuId);
        } else {
          gpuMissedSamples.current.set(gpuId, nextMissedSamples);
        }
      }
      const absentGpuIds = [...gpuMissedSamples.current.keys()].filter(
        (gpuId) => !currentGpuIdSet.has(gpuId),
      );

      setCpuHistory((prev) => padHistory([...prev, cpuUsage]));
      setMemoryHistory((prev) => padHistory([...prev, memoryUsage]));

      // CPU temperature (Windows thermal zones; null where unsupported)
      setCpuTemp(
        cpuTemperature != null ? [{ name: "CPU", value: cpuTemperature }] : [],
      );

      // All named temperature sensors (thermal zones)
      setSensorTemps(sensorTemperatures);
      setMotherboardTemps(motherboardTemperatures);
      setMotherboardFanSpeeds(motherboardFanSpeeds);
      const powerDraw = {
        cpuWatts: cpuPowerWatts,
        gpuWatts: gpuPowerWatts,
        aneWatts: anePowerWatts,
        packageWatts: packagePowerWatts,
      };
      setPowerDraw(powerDraw);
      if (Object.values(powerDraw).some((value) => value != null)) {
        setPowerDrawAvailable(true);
      }

      // Per-GPU usage histories
      setGpuHistories((prev) => {
        const next = { ...prev };

        for (const gpuId of retiredGpuIds) {
          delete next[gpuId];
        }
        for (const gpuId of absentGpuIds) {
          const history = next[gpuId];
          if (history != null) {
            next[gpuId] = padHistory([...history, null]);
          }
        }
        for (const gpu of gpus) {
          // The monitor-payload boundary: ids from the stream are branded
          // here. The other minting sites are the restored stored intent in
          // `useSelectedGpuPersistence` and the unresolved fallback in
          // `toLiveGpuId`; nothing else may mint.
          const gpuId = asLiveGpuId(gpu.gpuId);
          const history = next[gpuId];
          if (gpu.gpuUsage != null || history != null) {
            next[gpuId] = padHistory([...(history ?? []), gpu.gpuUsage]);
          }
        }

        return next;
      });

      // Temperature from all GPUs
      setGpuTempMap(
        liveGpuRecord(
          gpus
            .filter(
              (g): g is typeof g & { gpuTemperature: number } =>
                g.gpuTemperature != null,
            )
            .map((g) => [
              asLiveGpuId(g.gpuId),
              { name: g.gpuName, value: g.gpuTemperature },
            ]),
        ),
      );

      // Names from all GPUs. Every sample carries one, and it is the only way
      // to name an adapter: live ids and the inventory's ids do not share a
      // namespace on any platform (see `gpuNamesAtom`).
      // Merged, not replaced: a name is an identity, not a reading. A
      // provider that drops one adapter from a single sample must not erase
      // the only record that the adapter exists, or the Performance selector
      // would jump to another GPU on a transient hiccup.
      setGpuNames((prev) => {
        const next = {
          ...prev,
          ...liveGpuRecord(
            gpus.map((gpu) => [asLiveGpuId(gpu.gpuId), gpu.gpuName]),
          ),
        };
        for (const gpuId of retiredGpuIds) {
          delete next[gpuId];
        }
        return next;
      });

      // Usage sources from all GPUs
      setGpuSources(
        liveGpuRecord(
          gpus.map((gpu) => [asLiveGpuId(gpu.gpuId), gpu.gpuSource]),
        ),
      );

      // Dedicated memory is a per-sample reading. Replacing the map clears a
      // value when the adapter or the metric is absent instead of freezing it.
      setGpuMemoryMap(
        liveGpuRecord(
          gpus
            .filter(
              (g): g is typeof g & { gpuDedicatedMemoryUsageKb: number } =>
                g.gpuDedicatedMemoryUsageKb != null,
            )
            .map((gpu) => [
              asLiveGpuId(gpu.gpuId),
              gpu.gpuDedicatedMemoryUsageKb,
            ]),
        ),
      );

      // Fan speed from all GPUs
      setGpuFanSpeedMap(
        liveGpuRecord(
          gpus
            .filter(
              (g): g is typeof g & { gpuCoolerLevel: number } =>
                g.gpuCoolerLevel != null,
            )
            .map((g) => [
              asLiveGpuId(g.gpuId),
              { name: g.gpuName, value: g.gpuCoolerLevel },
            ]),
        ),
      );

      // Auto-select first GPU if none selected
      setSelectedGpuId((prev) =>
        prev != null
          ? prev
          : gpus.length > 0
            ? asLiveGpuId(gpus[0].gpuId)
            : null,
      );

      setProcessorsHistory((prev) => {
        const next = [...prev, processorsUsage];
        return next.slice(-chartConfig.historyLengthSec);
      });
    },
    [
      setCpuHistory,
      setMemoryHistory,
      setGpuHistories,
      setGpuTempMap,
      setProcessorsHistory,
      setGpuNames,
      setGpuSources,
      setGpuMemoryMap,
      setGpuFanSpeedMap,
      setSelectedGpuId,
      setCpuTemp,
      setSensorTemps,
      setMotherboardTemps,
      setMotherboardFanSpeeds,
      setPowerDrawAvailable,
      setPowerDraw,
    ],
  );
  useEffect(() => {
    const unlisten = events.hardwareMonitorUpdate.listen(handleHardwareUpdate);

    return () => {
      unlisten.then((off) => off());
    };
  }, [handleHardwareUpdate]);
};
