import { atom } from "jotai";
import { toProcessorRows } from "@/features/hardware/live/liveBuffers";
import {
  cpuUsageSeriesAtom,
  liveMetricsAtom,
  memoryUsageSeriesAtom,
} from "@/features/hardware/store/liveMetrics";

// ── Compatibility atoms ──
//
// The histories below used to be written by the monitor listener once a
// second. They are now derived from the Live Metrics Buffer
// (`store/liveMetrics.ts`) with the same names and shapes, so the screens that
// read them are unchanged; each is deleted once its last reader has moved to
// `useLiveSeries` (#1638, slice 3).

/** What a history reads as before the first sample arrives. */
const NO_SAMPLES: (number | null)[] = [];
const NO_ROWS: number[][] = [];

/** The CPU window, padded to its length once a sample has arrived. */
export const cpuUsageHistoryAtom = atom<(number | null)[]>((get) =>
  get(liveMetricsAtom).buffers.cpu.size === 0
    ? NO_SAMPLES
    : get(cpuUsageSeriesAtom),
);

/** The memory window, padded to its length once a sample has arrived. */
export const memoryUsageHistoryAtom = atom<(number | null)[]>((get) =>
  get(liveMetricsAtom).buffers.memory.size === 0
    ? NO_SAMPLES
    : get(memoryUsageSeriesAtom),
);

/**
 * Time-major per-core usage, oldest sample first and not padded: the matrix
 * the per-core series were split out of, rebuilt on every sample.
 */
export const processorsUsageHistoryAtom = atom<number[][]>((get) => {
  const { buffers } = get(liveMetricsAtom);
  return buffers.processorCounts.size === 0
    ? NO_ROWS
    : toProcessorRows(buffers);
});

// ── Derived scalars ──
//
// The history atoms above change on every 1 Hz sample, so a component that
// subscribes to one re-renders every second. These derive the one fact a
// reader actually needs; Jotai only notifies subscribers when the derived
// value changes, so a reader that wants a count or a flag does not pay for the
// series behind it (subscription granularity, #1638).

/**
 * Logical processor count as the System Specifications sheets report it: the
 * width of the oldest sample in the window. Changes at most once, when the
 * first sample arrives.
 */
export const processorCountAtom = atom(
  (get) => get(liveMetricsAtom).buffers.processorCounts.oldest() ?? 0,
);

/**
 * Logical processor count as the Performance strip reports it: the width of
 * the newest sample. Equal to `processorCountAtom` unless the stream changes
 * its processor count mid-window.
 */
export const latestProcessorCountAtom = atom(
  (get) => get(liveMetricsAtom).buffers.processorCounts.latest() ?? 0,
);

/**
 * Whether the monitor stream has delivered at least one sample. Panels use it
 * to tell "nothing has arrived yet" from "this sensor is absent".
 */
export const hasCpuUsageHistoryAtom = atom(
  (get) => get(liveMetricsAtom).buffers.cpu.size > 0,
);

/** Stand-in for `processorCountAtom` while runtime stats are disabled. */
export const disabledProcessorCountAtom = atom(0);
