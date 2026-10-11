import { atom } from "jotai";
import { selectAtom } from "jotai/utils";
import {
  type GpuAdapter,
  type GpuLiveMaps,
  getEffectiveGpuId,
  hasNoLiveGpuReadings,
  type LiveGpuId,
  listGpuAdapters,
  liveGpuRecord,
} from "@/features/hardware/gpuIdentity";
import type {
  LiveBuffers,
  LiveGpuBuffer,
} from "@/features/hardware/live/liveBuffers";
import {
  gpuUsageCurrentAtom,
  gpuUsageSeriesAtom,
  liveMetricsAtom,
  NO_LIVE_SAMPLES,
} from "@/features/hardware/store/liveMetrics";
import { selectedGpuIdAtom } from "@/features/hardware/store/selection";
import type { NameValues } from "@/features/hardware/types/hardwareDataType";
import { shallowEqualArray, shallowEqualRecord } from "@/lib/shallowEqual";
import type { NameValue } from "@/rspc/bindings";

// ── Multi-GPU state ──
//
// The per-GPU maps below are compatibility views of the Live Metrics Buffer
// (`store/liveMetrics.ts`): same names, same record shapes, rebuilt from the
// buffers on every sample. They are deleted once their last reader has moved
// to the live hooks (#1638, slice 4). `gpuNamesAtom` is not one of them; a
// name is an identity, written by the listener.

/** The adapters that reported in the latest sample, in payload order. */
const currentGpus = (buffers: LiveBuffers) =>
  buffers.currentGpuIds.flatMap((id) => {
    const entry = buffers.gpus.get(id);
    return entry != null ? [[id, entry] as const] : [];
  });

/**
 * One reading per adapter that carried it in the latest sample. A reading the
 * sample lacked leaves its key out, so the record clears a value when the
 * adapter or the metric is absent instead of freezing it.
 */
const currentReadings = <T>(
  buffers: LiveBuffers,
  pick: (entry: LiveGpuBuffer) => T | null,
) =>
  liveGpuRecord(
    currentGpus(buffers).flatMap(([id, entry]) => {
      const reading = pick(entry);
      return reading != null ? [[id, reading] as const] : [];
    }),
  );

/**
 * Per-GPU usage histories keyed by gpuId. Changes with every sample, so it is
 * not stabilised. An adapter has a key once it has reported usage.
 */
export const gpuUsageHistoriesAtom = atom<Record<LiveGpuId, (number | null)[]>>(
  (get) =>
    liveGpuRecord(
      [...get(liveMetricsAtom).buffers.gpus]
        .filter(([, entry]) => entry.usage.size > 0)
        .map(([id, entry]) => [id, entry.usage.toPaddedArray(null)] as const),
    ),
);

/**
 * Per-GPU name keyed by the live sampling id.
 *
 * The monitor stream and the one-shot `getHardwareInfo` inventory key their
 * GPUs in different namespaces on every platform — Windows NVIDIA reports the
 * raw NVAPI id as `GraphicInfo.id` but samples as `nvapi:<id>`, macOS pairs
 * `0x<registry_id>` with `iokit:<name>`, Linux pairs `card<n>` with the PCI
 * BDF. So a live id cannot be resolved against the inventory, and every
 * sample carries its own name for exactly that reason.
 */
export const gpuNamesAtom = atom<Record<LiveGpuId, string>>({});

// Rebuilt per sample but handed back unchanged when nothing moved, so a
// subscriber hears only about a changed reading.

/** Per-GPU usage source keyed by gpuId */
export const gpuUsageSourcesAtom = selectAtom(
  liveMetricsAtom,
  ({ buffers }): Record<LiveGpuId, string | null> =>
    liveGpuRecord(
      currentGpus(buffers).map(([id, entry]) => [id, entry.source]),
    ),
  (previous, next) => shallowEqualRecord(previous, next),
);

/** Per-GPU dedicated memory (KB) keyed by gpuId */
export const gpuDedicatedMemoryKbMapAtom = selectAtom(
  liveMetricsAtom,
  ({ buffers }): Record<LiveGpuId, number | null> =>
    currentReadings(buffers, (entry) => entry.dedicatedMemoryKb),
  (previous, next) => shallowEqualRecord(previous, next),
);

/** Per-GPU temperature keyed by gpuId */
export const gpuTempMapAtom = selectAtom(
  liveMetricsAtom,
  ({ buffers }): Record<LiveGpuId, NameValue> =>
    currentReadings(buffers, (entry) => entry.temperature),
  (previous, next) => shallowEqualRecord(previous, next, shallowEqualRecord),
);

/** Per-GPU fan speed keyed by gpuId */
export const gpuFanSpeedMapAtom = selectAtom(
  liveMetricsAtom,
  ({ buffers }): Record<LiveGpuId, NameValue> =>
    currentReadings(buffers, (entry) => entry.fanSpeed),
  (previous, next) => shallowEqualRecord(previous, next, shallowEqualRecord),
);

/** All GPUs temperature as NameValues */
export const gpuTempAtom = atom<NameValues>((get) =>
  Object.values(get(gpuTempMapAtom)),
);

/** All GPUs fan speed as NameValues */
export const gpuFanSpeedAtom = atom<NameValues>((get) =>
  Object.values(get(gpuFanSpeedMapAtom)),
);

// ── Derived atoms: the effective adapter and its values ──

/** The four live maps together, as the identity helpers expect them. */
const gpuLiveMapsAtom = atom<GpuLiveMaps>((get) => ({
  usageHistories: get(gpuUsageHistoriesAtom),
  temperatures: get(gpuTempMapAtom),
  fanSpeeds: get(gpuFanSpeedMapAtom),
  dedicatedMemoryKb: get(gpuDedicatedMemoryKbMapAtom),
}));

/**
 * The adapter every derived atom below describes.
 *
 * It has to be the same answer the GPU selectors show, or a surface would
 * label one adapter and render another's numbers. In particular an explicit
 * selection that reports no usage resolves to itself, so the consumers below
 * return nothing rather than borrowing the first adapter's values.
 *
 * A string (or `undefined`), so subscribers only hear about a change of
 * adapter, not about the readings it was resolved from.
 */
export const effectiveGpuIdAtom = atom<LiveGpuId | undefined>((get) =>
  getEffectiveGpuId(
    get(selectedGpuIdAtom),
    get(gpuLiveMapsAtom),
    Object.keys(get(gpuNamesAtom)) as LiveGpuId[],
  ),
);

const gpuAdapterCandidatesAtom = atom<GpuAdapter[]>((get) =>
  listGpuAdapters(get(gpuNamesAtom), get(gpuLiveMapsAtom)),
);

/**
 * Every adapter the Performance screens can attribute a reading to.
 *
 * Derived from the live maps, which change every second, but kept referentially
 * stable: a recomputation that lists the same adapters hands back the previous
 * array. Selectors and strips that only need the list therefore re-render when
 * an adapter appears, disappears or is renamed, not on every sample.
 */
export const gpuAdaptersAtom = selectAtom(
  gpuAdapterCandidatesAtom,
  (adapters) => adapters,
  (previous, next) => shallowEqualArray(previous, next, shallowEqualRecord),
);

/** The effective adapter's list entry. Stable while the adapter list is. */
export const effectiveGpuAdapterAtom = atom<GpuAdapter | undefined>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return get(gpuAdaptersAtom).find((adapter) => adapter.id === effective);
});

/**
 * Whether the effective adapter may be reported as having no live readings.
 * A flag, so the "unavailable" note does not re-render with every sample.
 */
export const gpuHasNoReadingsAtom = atom<boolean>((get) =>
  hasNoLiveGpuReadings(
    get(effectiveGpuIdAtom),
    get(gpuLiveMapsAtom),
    Object.keys(get(gpuNamesAtom)) as LiveGpuId[],
  ),
);

/**
 * The effective adapter's usage window, padded like every live series. All
 * gaps while there is no effective adapter or it has not reported usage; use
 * `gpuHasNoReadingsAtom` to tell "silent" from "not yet".
 */
export const effectiveGpuUsageSeriesAtom = atom<(number | null)[]>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null
    ? get(gpuUsageSeriesAtom(effective))
    : NO_LIVE_SAMPLES;
});

/**
 * The effective adapter's current usage, or `null` while there is none. A
 * primitive, so it notifies only when the reading changes.
 */
export const effectiveGpuUsageCurrentAtom = atom<number | null>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null ? get(gpuUsageCurrentAtom(effective)) : null;
});

/**
 * What `graphicUsageHistoryAtom` returns when the effective adapter has no
 * history. One shared array, so "still nothing" is not a new value every
 * sample.
 */
const NO_HISTORY: (number | null)[] = [];

/** Resolves to the effective GPU's usage history */
export const graphicUsageHistoryAtom = atom<(number | null)[]>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null
    ? (get(gpuUsageHistoriesAtom)[effective] ?? NO_HISTORY)
    : NO_HISTORY;
});

/** Resolves to the effective GPU's usage source */
export const gpuUsageSourceAtom = atom<string | null>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null
    ? (get(gpuUsageSourcesAtom)[effective] ?? null)
    : null;
});

/** Resolves to the effective GPU's dedicated memory (KB) */
export const gpuDedicatedMemoryKbAtom = atom<number | null>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null
    ? (get(gpuDedicatedMemoryKbMapAtom)[effective] ?? null)
    : null;
});

/** Resolves to the effective GPU's temperature value (as sampled, in the user's unit) */
export const gpuTemperatureValueAtom = atom<number | null>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null
    ? (get(gpuTempMapAtom)[effective]?.value ?? null)
    : null;
});

/** Resolves to the effective GPU's fan speed (cooler level) value */
export const gpuFanSpeedValueAtom = atom<number | null>((get) => {
  const effective = get(effectiveGpuIdAtom);
  return effective != null
    ? (get(gpuFanSpeedMapAtom)[effective]?.value ?? null)
    : null;
});
