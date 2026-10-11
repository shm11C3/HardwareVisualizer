import { chartConfig } from "@/consts/chart";
import type { GpuMonitorData, HardwareMonitorUpdate } from "@/rspc/bindings";

/**
 * Payload builders for tests that seed the Live Metrics Buffer. The buffer is
 * only written by publishing a monitor sample, so a test describes the sample
 * it wants and publishes it with `publishLiveSampleAtom`.
 */

/**
 * One adapter's slice of a sample. Everything but the id is absent unless
 * given, so a test names only the readings it cares about.
 */
export const liveGpu = (
  gpuId: string,
  overrides: Partial<Omit<GpuMonitorData, "gpuId" | "gpuSource">> & {
    /** The wire type is a string; `null` leaves the source unreported. */
    gpuSource?: string | null;
  } = {},
): GpuMonitorData => ({
  gpuId,
  gpuName: gpuId,
  gpuUsage: null,
  gpuTemperature: null,
  gpuDedicatedMemoryUsageKb: null,
  gpuCoolerLevel: null,
  ...overrides,
  gpuSource: (overrides.gpuSource === undefined
    ? null
    : overrides.gpuSource) as string,
});

/**
 * What a history atom reads as after `values` were the only samples: the
 * window padded with `null` on the left up to its length.
 */
export const paddedHistory = (
  ...values: (number | null)[]
): (number | null)[] => [
  ...Array<null>(chartConfig.historyLengthSec - values.length).fill(null),
  ...values,
];

type LiveSampleOverrides = Partial<
  Omit<HardwareMonitorUpdate, "cpuUsage" | "memoryUsage">
> & {
  /**
   * `null` marks a gap. The wire type is a number, but the buffer and the
   * charts treat a missing sample as a gap, and a test of that needs one.
   */
  cpuUsage?: number | null;
  memoryUsage?: number | null;
};

/** A monitor sample with no readings except the ones given. */
export const liveSample = (
  overrides: LiveSampleOverrides = {},
): HardwareMonitorUpdate => ({
  gpus: [],
  processorsUsage: [],
  cpuPowerWatts: null,
  gpuPowerWatts: null,
  anePowerWatts: null,
  packagePowerWatts: null,
  cpuPowerSupport: "unknown",
  cpuTemperature: null,
  sensorTemperatures: [],
  motherboardTemperatures: [],
  motherboardFanSpeeds: [],
  motherboardFanSupport: "unknown",
  ...overrides,
  cpuUsage: (overrides.cpuUsage === undefined
    ? 0
    : overrides.cpuUsage) as number,
  memoryUsage: (overrides.memoryUsage === undefined
    ? 0
    : overrides.memoryUsage) as number,
});
