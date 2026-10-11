import { asLiveGpuId, type LiveGpuId } from "@/features/hardware/gpuIdentity";
import {
  createLiveGpuBuffer,
  type LiveBuffers,
} from "@/features/hardware/live/liveBuffers";
import { RingBuffer } from "@/features/hardware/live/ringBuffer";
import type { PowerDraw } from "@/features/hardware/types/powerDraw";
import type { HardwareMonitorUpdate } from "@/rspc/bindings";

// One omitted sample can be a provider hiccup. Three consecutive visible
// samples establish that the adapter is no longer part of the live set while
// keeping unplug/fallback feedback within a few seconds at the 1 Hz cadence.
const GPU_RETIREMENT_MISSED_SAMPLES = 3;

const POWER_KEYS = [
  "cpuWatts",
  "gpuWatts",
  "aneWatts",
  "packageWatts",
] as const satisfies readonly (keyof PowerDraw)[];

/**
 * Append one monitor sample to the live buffers, in place.
 *
 * @param missingSampleCount Whole sampling intervals that passed without a
 *   delivered sample (the window was hidden). Only Power Draw history spans
 *   them with `null`s; the other series have always simply continued.
 * @returns The adapters this sample retired, so the caller can drop their
 *   names too.
 */
export const pushSample = (
  buffers: LiveBuffers,
  payload: HardwareMonitorUpdate,
  missingSampleCount: number,
): { retiredGpuIds: LiveGpuId[] } => {
  buffers.cpu.push(payload.cpuUsage);
  buffers.memory.push(payload.memoryUsage);
  pushProcessors(buffers, payload.processorsUsage);
  const retiredGpuIds = pushGpus(buffers, payload.gpus);
  pushPower(buffers, payload, missingSampleCount);
  return { retiredGpuIds };
};

const pushProcessors = (buffers: LiveBuffers, usage: readonly number[]) => {
  // Grow, never shrink. A sample with fewer cores pushes a gap into the
  // series it omits, so every series keeps the sample cadence and a per-core
  // chart never shows an old reading at the newest position.
  while (buffers.processors.length < usage.length) {
    buffers.processors.push(new RingBuffer<number | null>());
  }
  buffers.processors.forEach((series, index) => {
    series.push(usage[index] ?? null);
  });
  buffers.processorCounts.push(usage.length);
};

const pushGpus = (
  buffers: LiveBuffers,
  gpus: HardwareMonitorUpdate["gpus"],
): LiveGpuId[] => {
  // The monitor-payload boundary: ids from the stream are branded here. The
  // other minting sites are the restored stored intent in
  // `useSelectedGpuPersistence` and the unresolved fallback in `toLiveGpuId`;
  // nothing else may mint.
  const currentGpuIds = [...new Set(gpus.map((gpu) => asLiveGpuId(gpu.gpuId)))];
  const currentGpuIdSet = new Set(currentGpuIds);

  for (const gpuId of currentGpuIds) {
    let entry = buffers.gpus.get(gpuId);
    if (entry == null) {
      entry = createLiveGpuBuffer();
      buffers.gpus.set(gpuId, entry);
    }
    entry.missedSamples = 0;
  }

  const retiredGpuIds: LiveGpuId[] = [];
  for (const [gpuId, entry] of buffers.gpus) {
    if (currentGpuIdSet.has(gpuId)) {
      continue;
    }

    entry.missedSamples += 1;
    if (entry.missedSamples >= GPU_RETIREMENT_MISSED_SAMPLES) {
      retiredGpuIds.push(gpuId);
      buffers.gpus.delete(gpuId);
      continue;
    }

    // Still within the grace period: the usage window keeps its cadence with a
    // gap, and the per-sample readings are cleared because this sample has
    // none.
    if (entry.usage.size > 0) {
      entry.usage.push(null);
    }
    entry.temperature = null;
    entry.fanSpeed = null;
    entry.dedicatedMemoryKb = null;
    entry.source = null;
  }

  for (const gpu of gpus) {
    const gpuId = asLiveGpuId(gpu.gpuId);
    const entry = buffers.gpus.get(gpuId);
    if (entry == null) {
      continue;
    }

    // A `null` usage starts no history, but once one exists every sample
    // extends it, so the series is never shorter than the window.
    if (gpu.gpuUsage != null || entry.usage.size > 0) {
      const startsHistory = entry.usage.size === 0;
      entry.usage.push(gpu.gpuUsage);
      if (startsHistory) {
        buffers.gpus.delete(gpuId);
        buffers.gpus.set(gpuId, entry);
      }
    }
    entry.temperature =
      gpu.gpuTemperature != null
        ? { name: gpu.gpuName, value: gpu.gpuTemperature }
        : null;
    entry.fanSpeed =
      gpu.gpuCoolerLevel != null
        ? { name: gpu.gpuName, value: gpu.gpuCoolerLevel }
        : null;
    // Dedicated memory is a per-sample reading. Replacing it clears a value
    // when the adapter or the metric is absent instead of freezing it.
    entry.dedicatedMemoryKb = gpu.gpuDedicatedMemoryUsageKb;
    entry.source = gpu.gpuSource;
  }
  buffers.currentGpuIds = currentGpuIds;

  return retiredGpuIds;
};

const pushPower = (
  buffers: LiveBuffers,
  payload: HardwareMonitorUpdate,
  missingSampleCount: number,
) => {
  const current: PowerDraw = {
    cpuWatts: payload.cpuPowerWatts,
    gpuWatts: payload.gpuPowerWatts,
    aneWatts: payload.anePowerWatts,
    packageWatts: payload.packagePowerWatts,
  };
  buffers.power.current = current;

  const hasReading = POWER_KEYS.some((key) => current[key] != null);
  const historyStarted = POWER_KEYS.some(
    (key) => buffers.power.history[key].size > 0,
  );
  if (!hasReading && !historyStarted) {
    return;
  }

  // Keep the graph's one-sample-per-second positions honest when delivery
  // resumes after a hidden window instead of presenting pre-hide values as
  // recent.
  for (const key of POWER_KEYS) {
    const history = buffers.power.history[key];
    for (let missed = 0; missed < missingSampleCount; missed += 1) {
      history.push(null);
    }
    history.push(current[key]);
  }
};
