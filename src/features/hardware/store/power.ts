import { atom } from "jotai";
import { selectAtom } from "jotai/utils";
import type { RingBuffer } from "@/features/hardware/live/ringBuffer";
import { liveMetricsAtom } from "@/features/hardware/store/liveMetrics";
import type {
  PowerDraw,
  PowerDrawHistory,
} from "@/features/hardware/types/powerDraw";
import { shallowEqualRecord } from "@/lib/shallowEqual";
import type { SensorSupport } from "@/rspc/bindings";

// The two atoms below are compatibility views of the Live Metrics Buffer
// (`store/liveMetrics.ts`), kept until their readers move to the live hooks
// (#1638, slice 3).

/**
 * The latest power reading. Recomputed with every sample but kept
 * referentially stable, so a subscriber hears only about a changed value.
 */
export const powerDrawAtom = selectAtom(
  liveMetricsAtom,
  ({ buffers }): PowerDraw => buffers.power.current,
  shallowEqualRecord,
);

const seriesOf = (series: RingBuffer<number | null>) =>
  series.size === 0 ? NO_SAMPLES : series.toPaddedArray(null);

/** What a power series reads as until the first reading has arrived. */
const NO_SAMPLES: (number | null)[] = [];

const POWER_KEYS = [
  "cpuWatts",
  "gpuWatts",
  "aneWatts",
  "packageWatts",
] as const satisfies readonly (keyof PowerDraw)[];

/**
 * The power windows, each padded to its length once the history has started.
 * Before that every series is the same empty array, so the object is stable
 * and a subscriber is not woken once a second for nothing.
 */
export const powerDrawHistoryAtom = selectAtom(
  liveMetricsAtom,
  ({ buffers }): PowerDrawHistory => {
    const { history } = buffers.power;
    return {
      cpuWatts: seriesOf(history.cpuWatts),
      gpuWatts: seriesOf(history.gpuWatts),
      aneWatts: seriesOf(history.aneWatts),
      packageWatts: seriesOf(history.packageWatts),
    };
  },
  (previous, next) => POWER_KEYS.every((key) => previous[key] === next[key]),
);

/** Whether this runtime has produced at least one power reading. */
export const powerDrawAvailableAtom = atom(false);

/** Hardware support for CPU package-power collection. */
export const cpuPowerSupportAtom = atom<SensorSupport>("unknown");
