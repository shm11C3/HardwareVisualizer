import { useAtom, useAtomValue } from "jotai";
import { useMemo } from "react";
import {
  type GpuLiveMaps,
  getEffectiveGpuId,
  hasNoLiveGpuReadings,
  listGpuAdapters,
} from "@/features/hardware/gpuIdentity";
import {
  gpuDedicatedMemoryKbMapAtom,
  gpuFanSpeedMapAtom,
  gpuNamesAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";

/**
 * The one place the GPU surfaces agree on which adapter they are describing.
 *
 * Every view that renders a GPU reading needs the same three answers — which
 * adapters exist, which one is effective, and whether it is reporting — and
 * they have to be the same answers, or two views would attribute the same
 * numbers to different devices.
 */
export const useGpuAdapters = () => {
  const gpuUsageHistories = useAtomValue(gpuUsageHistoriesAtom);
  const gpuTemperatureMap = useAtomValue(gpuTempMapAtom);
  const gpuFanSpeedMap = useAtomValue(gpuFanSpeedMapAtom);
  const gpuDedicatedMemoryKbMap = useAtomValue(gpuDedicatedMemoryKbMapAtom);
  const gpuNames = useAtomValue(gpuNamesAtom);
  const [selectedGpuId, setSelectedGpuId] = useAtom(selectedGpuIdAtom);

  const live = useMemo<GpuLiveMaps>(
    () => ({
      usageHistories: gpuUsageHistories,
      temperatures: gpuTemperatureMap,
      fanSpeeds: gpuFanSpeedMap,
      dedicatedMemoryKb: gpuDedicatedMemoryKbMap,
    }),
    [
      gpuUsageHistories,
      gpuTemperatureMap,
      gpuFanSpeedMap,
      gpuDedicatedMemoryKbMap,
    ],
  );

  const adapters = useMemo(
    () => listGpuAdapters(gpuNames, live),
    [gpuNames, live],
  );
  const detectedGpuIds = useMemo(
    () => adapters.map((adapter) => adapter.id),
    [adapters],
  );
  const effectiveGpuId = getEffectiveGpuId(selectedGpuId, live, detectedGpuIds);

  return {
    live,
    adapters,
    effectiveGpuId,
    effectiveAdapter: adapters.find((adapter) => adapter.id === effectiveGpuId),
    hasNoReadings: hasNoLiveGpuReadings(effectiveGpuId, live, detectedGpuIds),
    selectGpu: setSelectedGpuId,
  };
};
