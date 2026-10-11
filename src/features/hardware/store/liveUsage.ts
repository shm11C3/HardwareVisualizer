import { atom } from "jotai";
import { liveMetricsAtom } from "@/features/hardware/store/liveMetrics";

// ── Derived scalars ──
//
// A series changes on every 1 Hz sample, so a component that subscribes to one
// re-renders every second. These derive the one fact a reader actually needs
// from the Live Metrics Buffer; Jotai only notifies subscribers when the
// derived value changes, so a reader that wants a count or a flag does not pay
// for the series behind it (subscription granularity, #1638).

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
