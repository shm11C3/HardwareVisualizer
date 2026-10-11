import type { LiveGpuId } from "@/features/hardware/gpuIdentity";
import { RingBuffer } from "@/features/hardware/live/ringBuffer";
import type { PowerDraw } from "@/features/hardware/types/powerDraw";
import type { NameValue } from "@/rspc/bindings";

/** One adapter's live channel: a usage series plus the latest readings. */
export type LiveGpuBuffer = {
  /**
   * Empty until the adapter reports its first usage. After that every sample
   * appends, `null` included, so a gap stays a gap instead of shortening the
   * window.
   */
  usage: RingBuffer<number | null>;
  /** Latest readings. `null` when the latest sample did not carry one. */
  temperature: NameValue | null;
  fanSpeed: NameValue | null;
  dedicatedMemoryKb: number | null;
  source: string | null;
  /**
   * Consecutive samples that omitted this adapter. The adapter is dropped
   * after `GPU_RETIREMENT_MISSED_SAMPLES`; see `pushSample`.
   */
  missedSamples: number;
};

export type PowerDrawBuffers = {
  [K in keyof PowerDraw]: RingBuffer<number | null>;
};

/**
 * Everything the monitor stream keeps for the live window, owned by one Jotai
 * store (see `store/liveMetrics.ts`). Mutated in place by `pushSample`; nothing
 * else writes to it.
 */
export type LiveBuffers = {
  cpu: RingBuffer<number | null>;
  memory: RingBuffer<number | null>;
  /**
   * One series per logical processor, indexed by processor. Every sample
   * pushes into every series that exists at that point: a value for the cores
   * it carries and `null` for the cores it omits, so the series stay aligned
   * in time and a narrower sample shows as a gap, never as a shifted or stale
   * reading. `processorCounts` says how wide each sample was.
   */
  processors: RingBuffer<number | null>[];
  /** Width of each retained sample, so a time-major view can be rebuilt. */
  processorCounts: RingBuffer<number>;
  /**
   * Adapters seen and not yet retired. Insertion order matters to readers
   * that list adapters, so an entry moves to the end when its usage history
   * starts, which is when the old per-adapter history map created its key.
   */
  gpus: Map<LiveGpuId, LiveGpuBuffer>;
  /** Adapters in the latest sample, in payload order. */
  currentGpuIds: LiveGpuId[];
  power: {
    current: PowerDraw;
    history: PowerDrawBuffers;
  };
};

export const createLiveGpuBuffer = (): LiveGpuBuffer => ({
  usage: new RingBuffer<number | null>(),
  temperature: null,
  fanSpeed: null,
  dedicatedMemoryKb: null,
  source: null,
  missedSamples: 0,
});

export const createLiveBuffers = (): LiveBuffers => ({
  cpu: new RingBuffer<number | null>(),
  memory: new RingBuffer<number | null>(),
  processors: [],
  processorCounts: new RingBuffer<number>(),
  gpus: new Map(),
  currentGpuIds: [],
  power: {
    current: {
      cpuWatts: null,
      gpuWatts: null,
      aneWatts: null,
      packageWatts: null,
    },
    history: {
      cpuWatts: new RingBuffer<number | null>(),
      gpuWatts: new RingBuffer<number | null>(),
      aneWatts: new RingBuffer<number | null>(),
      packageWatts: new RingBuffer<number | null>(),
    },
  },
});

/**
 * The retained samples as rows of per-core usage, oldest first: the
 * time-by-core matrix the per-core series were split out of. A row is as wide
 * as the sample it came from.
 */
export const toProcessorRows = (buffers: LiveBuffers): number[][] => {
  const widths = buffers.processorCounts.toArray();
  const cores = buffers.processors.map((core) => core.toArray());
  // Every series is right-aligned on the newest sample, and a core that a
  // sample carried has received an entry for every sample since, so the row
  // `j` samples back reads each core `j` entries from its end.
  return widths.map((width, row) => {
    const fromEnd = widths.length - row;
    return Array.from(
      { length: width },
      (_, index) => cores[index]?.at(-fromEnd) as number,
    );
  });
};
