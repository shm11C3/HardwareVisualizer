import { type Atom, atom } from "jotai";
import { chartConfig } from "@/consts/chart";
import type { LiveGpuId } from "@/features/hardware/gpuIdentity";
import { createLiveBuffers } from "@/features/hardware/live/liveBuffers";
import { pushSample } from "@/features/hardware/live/pushSample";
import type { RingBuffer } from "@/features/hardware/live/ringBuffer";
import type { HardwareMonitorUpdate } from "@/rspc/bindings";

// ── The Live Metrics Buffer (#1638) ──
//
// The monitor stream is buffered here instead of being written into one atom
// per channel. The buffers are fixed-length ring buffers that `pushSample`
// mutates in place, so a sample allocates nothing per channel; one version
// bump per sample tells Jotai that something changed, and the derived atoms
// below are the only readers. A derived atom that produces a primitive (or a
// value kept referentially stable) does not notify its subscribers when a
// sample leaves it unchanged.
//
// The buffers live in the Jotai store, not in the module: `liveBuffersAtom`
// is a read-only atom without dependencies, which Jotai evaluates once per
// store. Remounting the window's `Provider` (error-boundary recovery) or
// creating a store in a test therefore starts from empty buffers, where a
// module-level object would survive both.

/** Created once per Jotai store, on first read. Never read it outside this module. */
const liveBuffersAtom = atom(() => createLiveBuffers());

const liveVersionAtom = atom(0);

/**
 * The buffers at a given version. Each version is a new object, because Jotai
 * decides whether a derived atom changed by `Object.is` on its value: handing
 * back the (mutated, therefore identical) buffers object would never notify
 * anyone.
 */
export const liveMetricsAtom = atom((get) => ({
  version: get(liveVersionAtom),
  buffers: get(liveBuffersAtom),
}));

/**
 * Append one monitor sample. This is the listener's only write for everything
 * the buffers own. Resolves to the adapters the sample retired, so the caller
 * can drop their names.
 */
export const publishLiveSampleAtom = atom(
  null,
  (
    get,
    set,
    payload: HardwareMonitorUpdate,
    missingSampleCount: number,
  ): { retiredGpuIds: LiveGpuId[] } => {
    const result = pushSample(
      get(liveBuffersAtom),
      payload,
      missingSampleCount,
    );
    set(liveVersionAtom, (version) => version + 1);
    return result;
  },
);

/**
 * Forget every GPU temperature. Readings already on screen are in the previous
 * temperature unit; the next sample replaces them.
 */
export const clearGpuTemperaturesAtom = atom(null, (get, set) => {
  for (const entry of get(liveBuffersAtom).gpus.values()) {
    entry.temperature = null;
  }
  set(liveVersionAtom, (version) => version + 1);
});

// ── Scalars ──
//
// Primitives, so a sample that does not change the value notifies nobody.

export const cpuUsageCurrentAtom = atom<number | null>(
  (get) => get(liveMetricsAtom).buffers.cpu.latest() ?? null,
);

export const memoryUsageCurrentAtom = atom<number | null>(
  (get) => get(liveMetricsAtom).buffers.memory.latest() ?? null,
);

// ── Series ──
//
// Padded here, and only here, to the window length: oldest first, `null`
// where no sample has arrived yet. A fresh array per sample, because the
// window slid.

const padded = (series: RingBuffer<number | null> | undefined) =>
  series != null
    ? series.toPaddedArray(null)
    : Array<null>(chartConfig.historyLengthSec).fill(null);

export const cpuUsageSeriesAtom = atom<(number | null)[]>((get) =>
  padded(get(liveMetricsAtom).buffers.cpu),
);

export const memoryUsageSeriesAtom = atom<(number | null)[]>((get) =>
  padded(get(liveMetricsAtom).buffers.memory),
);

// jotai 3 has no `atomFamily`. These caches hold one atom per key, and the key
// set is bounded by the hardware (cores, adapters), so they do not grow with
// time. They hold atoms (configuration), not state: the values live in the
// store.
const processorSeriesAtoms = new Map<number, Atom<(number | null)[]>>();
const gpuSeriesAtoms = new Map<LiveGpuId, Atom<(number | null)[]>>();

/** One logical processor's usage series. Empty-padded before it reports. */
export const processorUsageSeriesAtom = (index: number) => {
  let seriesAtom = processorSeriesAtoms.get(index);
  if (seriesAtom == null) {
    seriesAtom = atom((get) =>
      padded(get(liveMetricsAtom).buffers.processors[index]),
    );
    processorSeriesAtoms.set(index, seriesAtom);
  }
  return seriesAtom;
};

/** One adapter's usage series. All `null` before it reports usage. */
export const gpuUsageSeriesAtom = (id: LiveGpuId) => {
  let seriesAtom = gpuSeriesAtoms.get(id);
  if (seriesAtom == null) {
    seriesAtom = atom((get) =>
      padded(get(liveMetricsAtom).buffers.gpus.get(id)?.usage),
    );
    gpuSeriesAtoms.set(id, seriesAtom);
  }
  return seriesAtom;
};

export type LiveSeriesChannel =
  | { kind: "cpu" }
  | { kind: "memory" }
  | { kind: "processor"; index: number }
  | { kind: "gpu"; id: LiveGpuId };

/** The series atom for a channel. Atoms for the same channel are the same atom. */
export const liveSeriesAtom = (
  channel: LiveSeriesChannel,
): Atom<(number | null)[]> => {
  switch (channel.kind) {
    case "cpu":
      return cpuUsageSeriesAtom;
    case "memory":
      return memoryUsageSeriesAtom;
    case "processor":
      return processorUsageSeriesAtom(channel.index);
    case "gpu":
      return gpuUsageSeriesAtom(channel.id);
  }
};

export type LiveScalarChannel = "cpu" | "memory";

/** The current-value atom for a channel. */
export const liveScalarAtom = (
  channel: LiveScalarChannel,
): Atom<number | null> =>
  channel === "cpu" ? cpuUsageCurrentAtom : memoryUsageCurrentAtom;
