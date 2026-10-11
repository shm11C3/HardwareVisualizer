import { useSetAtom } from "jotai";
import { useCallback, useEffect, useRef } from "react";
import { chartConfig } from "@/consts/chart";
import { asLiveGpuId, liveGpuRecord } from "@/features/hardware/gpuIdentity";
import { gpuNamesAtom } from "@/features/hardware/store/gpu";
import {
  clearGpuTemperaturesAtom,
  publishLiveSampleAtom,
} from "@/features/hardware/store/liveMetrics";
import {
  cpuPowerSupportAtom,
  powerDrawAvailableAtom,
} from "@/features/hardware/store/power";
import { selectedGpuIdAtom } from "@/features/hardware/store/selection";
import {
  cpuTempAtom,
  motherboardFanSpeedsAtom,
  motherboardFanSupportAtom,
  motherboardTempsAtom,
  sensorTempsAtom,
} from "@/features/hardware/store/sensors";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { shallowEqualArray, shallowEqualRecord } from "@/lib/shallowEqual";
import {
  events,
  type HardwareMonitorUpdate,
  type TemperatureUnit,
} from "@/rspc/bindings";

/**
 * Functional update for a value the listener rebuilds on every tick.
 *
 * Jotai skips notification when the written value is `Object.is`-equal, so
 * handing the previous reference back for an unchanged value is what stops
 * its subscribers from re-rendering once a second. The high-frequency live
 * channels do not go through here: they are appended to the Live Metrics
 * Buffer (`store/liveMetrics.ts`), whose derived atoms do the same.
 */
const keepArray =
  <T>(next: readonly T[], itemEqual?: (a: T, b: T) => boolean) =>
  (previous: T[]): T[] =>
    shallowEqualArray(previous, next, itemEqual) ? previous : [...next];

const MONITOR_SAMPLE_INTERVAL_MS = 1000;

const getMissingSampleCount = (elapsedMs: number): number =>
  Math.min(
    Math.max(Math.round(elapsedMs / MONITOR_SAMPLE_INTERVAL_MS) - 1, 0),
    chartConfig.historyLengthSec - 1,
  );

/**
 * Listen for hardware monitor update events pushed from the backend.
 * Replaces the 4x useUsageUpdater polling hooks with a single event listener.
 */
export const useHardwareEventListener = () => {
  const lastVisibleUpdateAt = useRef<number | null>(null);
  const { settings } = useSettingsAtom();
  const lastTemperatureUnit = useRef<TemperatureUnit | null>(null);
  const publishLiveSample = useSetAtom(publishLiveSampleAtom);
  const clearGpuTemperatures = useSetAtom(clearGpuTemperaturesAtom);
  const setGpuNames = useSetAtom(gpuNamesAtom);
  const setSelectedGpuId = useSetAtom(selectedGpuIdAtom);
  const setCpuTemp = useSetAtom(cpuTempAtom);
  const setSensorTemps = useSetAtom(sensorTempsAtom);
  const setMotherboardTemps = useSetAtom(motherboardTempsAtom);
  const setMotherboardFanSpeeds = useSetAtom(motherboardFanSpeedsAtom);
  const setMotherboardFanSupport = useSetAtom(motherboardFanSupportAtom);
  const setCpuPowerSupport = useSetAtom(cpuPowerSupportAtom);
  const setPowerDrawAvailable = useSetAtom(powerDrawAvailableAtom);

  const handleHardwareUpdate = useCallback(
    (event: { payload: HardwareMonitorUpdate }) => {
      if (document.hidden) {
        return;
      }

      const updateReceivedAt = Date.now();
      // The App stops forwarding snapshots while the main window is hidden.
      // Keep the Power Draw graph's one-sample-per-second positions honest
      // when delivery resumes instead of presenting pre-hide values as recent.
      const missingSampleCount =
        lastVisibleUpdateAt.current == null
          ? 0
          : getMissingSampleCount(
              updateReceivedAt - lastVisibleUpdateAt.current,
            );
      lastVisibleUpdateAt.current = updateReceivedAt;

      const {
        gpus,
        cpuTemperature,
        sensorTemperatures,
        motherboardTemperatures,
        motherboardFanSpeeds,
        cpuPowerWatts,
        gpuPowerWatts,
        anePowerWatts,
        packagePowerWatts,
        cpuPowerSupport,
        motherboardFanSupport,
      } = event.payload;

      // Usage, power and per-adapter readings are appended to the Live Metrics
      // Buffer in one write. It also decides which adapters have been absent
      // long enough to retire.
      const { retiredGpuIds } = publishLiveSample(
        event.payload,
        missingSampleCount,
      );

      // CPU temperature (Windows thermal zones; null where unsupported)
      setCpuTemp(
        keepArray(
          cpuTemperature != null
            ? [{ name: "CPU", value: cpuTemperature }]
            : [],
          shallowEqualRecord,
        ),
      );

      // All named temperature sensors (thermal zones)
      setSensorTemps(keepArray(sensorTemperatures, shallowEqualRecord));
      setMotherboardTemps(
        keepArray(motherboardTemperatures, shallowEqualRecord),
      );
      setMotherboardFanSpeeds(
        keepArray(motherboardFanSpeeds, shallowEqualRecord),
      );
      setMotherboardFanSupport(motherboardFanSupport);
      setCpuPowerSupport(cpuPowerSupport);
      const hasPowerReading = [
        cpuPowerWatts,
        gpuPowerWatts,
        anePowerWatts,
        packagePowerWatts,
      ].some((value) => value != null);
      if (hasPowerReading) {
        setPowerDrawAvailable(true);
      }

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
        // Names rarely change, and everything that lists adapters hangs off
        // this reference.
        return shallowEqualRecord(prev, next) ? prev : next;
      });

      // Auto-select first GPU if none selected
      setSelectedGpuId((prev) =>
        prev != null
          ? prev
          : gpus.length > 0
            ? asLiveGpuId(gpus[0].gpuId)
            : null,
      );
    },
    [
      publishLiveSample,
      setGpuNames,
      setSelectedGpuId,
      setCpuTemp,
      setSensorTemps,
      setMotherboardTemps,
      setMotherboardFanSpeeds,
      setMotherboardFanSupport,
      setCpuPowerSupport,
      setPowerDrawAvailable,
    ],
  );

  // GPU temperatures already on screen are in the previous unit. Clear them
  // when the unit changes so they are not shown until the next 1 Hz sample
  // replaces them. The first observed unit is the baseline, not a change.
  const { temperatureUnit } = settings;
  useEffect(() => {
    const previousUnit = lastTemperatureUnit.current;
    lastTemperatureUnit.current = temperatureUnit;
    if (previousUnit !== null && previousUnit !== temperatureUnit) {
      clearGpuTemperatures();
    }
  }, [temperatureUnit, clearGpuTemperatures]);

  useEffect(() => {
    const unlisten = events.hardwareMonitorUpdate.listen(handleHardwareUpdate);

    return () => {
      unlisten.then((off) => off());
    };
  }, [handleHardwareUpdate]);
};
