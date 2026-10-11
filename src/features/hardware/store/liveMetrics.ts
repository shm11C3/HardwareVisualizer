import { type Atom, atom } from "jotai";
import { selectAtom } from "jotai/utils";
import { chartConfig } from "@/consts/chart";
import type { LiveGpuId } from "@/features/hardware/gpuIdentity";
import { createLiveBuffers } from "@/features/hardware/live/liveBuffers";
import { pushSample } from "@/features/hardware/live/pushSample";
import type { RingBuffer } from "@/features/hardware/live/ringBuffer";
import type { PowerDraw } from "@/features/hardware/types/powerDraw";
import { shallowEqualArray } from "@/lib/shallowEqual";
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

// jotai 3 has no `atomFamily`. These caches hold one atom per key, and the key
// set is bounded by the hardware (cores, adapters, power domains), so they do
// not grow with time. They hold atoms (configuration), not state: the values
// live in the store.
const gpuCurrentAtoms = new Map<LiveGpuId, Atom<number | null>>();
const powerCurrentAtoms = new Map<keyof PowerDraw, Atom<number | null>>();

/** One adapter's current usage. `null` until it reports, and for a `null` sample. */
export const gpuUsageCurrentAtom = (id: LiveGpuId) => {
  let currentAtom = gpuCurrentAtoms.get(id);
  if (currentAtom == null) {
    currentAtom = atom(
      (get) =>
        get(liveMetricsAtom).buffers.gpus.get(id)?.usage.latest() ?? null,
    );
    gpuCurrentAtoms.set(id, currentAtom);
  }
  return currentAtom;
};

/** One power domain's latest reading, in watts. `null` where nothing is reported. */
export const powerCurrentAtom = (key: keyof PowerDraw) => {
  let currentAtom = powerCurrentAtoms.get(key);
  if (currentAtom == null) {
    currentAtom = atom(
      (get) => get(liveMetricsAtom).buffers.power.current[key],
    );
    powerCurrentAtoms.set(key, currentAtom);
  }
  return currentAtom;
};

/**
 * The newest sample's per-core usage, one entry per logical processor, or
 * `null` before any sample has arrived. An empty array means a sample arrived
 * without per-core data. Handed back unchanged when no core moved.
 */
export const latestProcessorUsagesAtom = selectAtom(
  liveMetricsAtom,
  ({ buffers }): number[] | null => {
    const width = buffers.processorCounts.latest();
    return width == null
      ? null
      : Array.from(
          { length: width },
          (_, index) => buffers.processors[index]?.latest() as number,
        );
  },
  (previous, next) =>
    previous === next ||
    (previous != null && next != null && shallowEqualArray(previous, next)),
);

// ── Series ──
//
// Padded here, and only here, to the window length: oldest first, `null`
// where no sample has arrived yet. A fresh array per sample, because the
// window slid.

/**
 * What a series reads as before it has a single sample: the window, all gaps.
 * One shared array, so "still nothing" is not a new value every sample and its
 * readers are not woken for it. Nobody may mutate it.
 */
export const NO_LIVE_SAMPLES: (number | null)[] = Array<null>(
  chartConfig.historyLengthSec,
).fill(null);

const padded = (series: RingBuffer<number | null> | undefined) =>
  series == null || series.size === 0
    ? NO_LIVE_SAMPLES
    : series.toPaddedArray(null);

export const cpuUsageSeriesAtom = atom<(number | null)[]>((get) =>
  padded(get(liveMetricsAtom).buffers.cpu),
);

export const memoryUsageSeriesAtom = atom<(number | null)[]>((get) =>
  padded(get(liveMetricsAtom).buffers.memory),
);

const processorSeriesAtoms = new Map<number, Atom<(number | null)[]>>();
const gpuSeriesAtoms = new Map<LiveGpuId, Atom<(number | null)[]>>();
const powerSeriesAtoms = new Map<keyof PowerDraw, Atom<(number | null)[]>>();

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

/** One power domain's series, in watts. All `null` until a reading arrives. */
export const powerDrawSeriesAtom = (key: keyof PowerDraw) => {
  let seriesAtom = powerSeriesAtoms.get(key);
  if (seriesAtom == null) {
    seriesAtom = atom((get) =>
      padded(get(liveMetricsAtom).buffers.power.history[key]),
    );
    powerSeriesAtoms.set(key, seriesAtom);
  }
  return seriesAtom;
};

export type LiveSeriesChannel =
  | { kind: "cpu" }
  | { kind: "memory" }
  | { kind: "processor"; index: number }
  | { kind: "gpu"; id: LiveGpuId }
  | { kind: "power"; key: keyof PowerDraw };

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
    case "power":
      return powerDrawSeriesAtom(channel.key);
  }
};

export type LiveScalarChannel =
  | "cpu"
  | "memory"
  | { kind: "gpu"; id: LiveGpuId }
  | { kind: "power"; key: keyof PowerDraw };

/** The current-value atom for a channel. */
export const liveScalarAtom = (
  channel: LiveScalarChannel,
): Atom<number | null> => {
  if (channel === "cpu") {
    return cpuUsageCurrentAtom;
  }
  if (channel === "memory") {
    return memoryUsageCurrentAtom;
  }
  return channel.kind === "gpu"
    ? gpuUsageCurrentAtom(channel.id)
    : powerCurrentAtom(channel.key);
};
