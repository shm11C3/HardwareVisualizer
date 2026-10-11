import { describe, expect, it } from "vitest";
import { chartConfig } from "@/consts/chart";
import { buildHardwareUpdateSeries } from "@/e2e/fixtures/hardware";
import { asLiveGpuId } from "@/features/hardware/gpuIdentity";
import {
  createLiveBuffers,
  toProcessorRows,
} from "@/features/hardware/live/liveBuffers";
import {
  liveGpu,
  liveSample,
} from "@/features/hardware/live/liveSamples.testHelpers";
import { pushSample } from "@/features/hardware/live/pushSample";

const windowLength = chartConfig.historyLengthSec;
const gpuA = asLiveGpuId("gpu-a");
const gpuB = asLiveGpuId("gpu-b");

describe("pushSample with the fixture series", () => {
  it("keeps the newest window of every channel and nothing more", () => {
    const series = buildHardwareUpdateSeries(windowLength + 15);
    const buffers = createLiveBuffers();

    for (const sample of series) {
      pushSample(buffers, sample, 0);
    }

    const newest = series.slice(-windowLength);
    expect(buffers.cpu.toArray()).toEqual(newest.map((s) => s.cpuUsage));
    expect(buffers.memory.toArray()).toEqual(newest.map((s) => s.memoryUsage));
    expect(toProcessorRows(buffers)).toEqual(
      newest.map((s) => s.processorsUsage),
    );
    for (const gpu of [0, 1]) {
      const id = asLiveGpuId(series[0].gpus[gpu].gpuId);
      expect(buffers.gpus.get(id)?.usage.toArray()).toEqual(
        newest.map((s) => s.gpus[gpu].gpuUsage),
      );
    }
    expect(buffers.power.history.cpuWatts.toArray()).toEqual(
      newest.map((s) => s.cpuPowerWatts),
    );
    expect(buffers.power.current).toEqual({
      cpuWatts: series.at(-1)?.cpuPowerWatts,
      gpuWatts: series.at(-1)?.gpuPowerWatts,
      aneWatts: series.at(-1)?.anePowerWatts,
      packageWatts: series.at(-1)?.packagePowerWatts,
    });
  });

  it("holds the latest per-adapter readings, in payload order", () => {
    const series = buildHardwareUpdateSeries(3);
    const buffers = createLiveBuffers();

    for (const sample of series) {
      pushSample(buffers, sample, 0);
    }

    const last = series.at(-1)?.gpus ?? [];
    expect(buffers.currentGpuIds).toEqual(
      last.map((gpu) => asLiveGpuId(gpu.gpuId)),
    );
    for (const gpu of last) {
      const entry = buffers.gpus.get(asLiveGpuId(gpu.gpuId));
      expect(entry?.temperature).toEqual({
        name: gpu.gpuName,
        value: gpu.gpuTemperature,
      });
      expect(entry?.fanSpeed).toEqual({
        name: gpu.gpuName,
        value: gpu.gpuCoolerLevel,
      });
      expect(entry?.dedicatedMemoryKb).toBe(gpu.gpuDedicatedMemoryUsageKb);
      expect(entry?.source).toBe(gpu.gpuSource);
    }
  });
});

describe("pushSample processors", () => {
  it("grows the per-core series when a sample has more cores and never shrinks", () => {
    const buffers = createLiveBuffers();

    pushSample(buffers, liveSample({ processorsUsage: [1, 2] }), 0);
    pushSample(buffers, liveSample({ processorsUsage: [3, 4, 5, 6] }), 0);
    pushSample(buffers, liveSample({ processorsUsage: [7] }), 0);

    expect(buffers.processors).toHaveLength(4);
    expect(toProcessorRows(buffers)).toEqual([[1, 2], [3, 4, 5, 6], [7]]);
  });

  it("rebuilds rows of the width each sample had once the window slides", () => {
    const buffers = createLiveBuffers();
    pushSample(buffers, liveSample({ processorsUsage: [1, 2, 3] }), 0);
    for (let i = 0; i < windowLength - 1; i += 1) {
      pushSample(buffers, liveSample({ processorsUsage: [10] }), 0);
    }
    expect(toProcessorRows(buffers)[0]).toEqual([1, 2, 3]);

    pushSample(buffers, liveSample({ processorsUsage: [10] }), 0);

    expect(toProcessorRows(buffers)).toHaveLength(windowLength);
    expect(toProcessorRows(buffers).every((row) => row.length === 1)).toBe(
      true,
    );
  });
});

describe("pushSample adapters", () => {
  it("starts no history for a null usage, then extends it with every sample", () => {
    const buffers = createLiveBuffers();

    pushSample(buffers, liveSample({ gpus: [liveGpu("gpu-a")] }), 0);
    expect(buffers.gpus.get(gpuA)?.usage.size).toBe(0);

    pushSample(
      buffers,
      liveSample({ gpus: [liveGpu("gpu-a", { gpuUsage: 40 })] }),
      0,
    );
    pushSample(buffers, liveSample({ gpus: [liveGpu("gpu-a")] }), 0);

    expect(buffers.gpus.get(gpuA)?.usage.toArray()).toEqual([40, null]);
  });

  it("opens a gap for an adapter that is absent, and clears its readings", () => {
    const buffers = createLiveBuffers();
    pushSample(
      buffers,
      liveSample({
        gpus: [
          liveGpu("gpu-a", {
            gpuUsage: 40,
            gpuTemperature: 60,
            gpuCoolerLevel: 30,
            gpuDedicatedMemoryUsageKb: 1024,
            gpuSource: "NVAPI",
          }),
        ],
      }),
      0,
    );

    pushSample(buffers, liveSample({ gpus: [] }), 0);

    const entry = buffers.gpus.get(gpuA);
    expect(entry?.usage.toArray()).toEqual([40, null]);
    expect(entry?.temperature).toBeNull();
    expect(entry?.fanSpeed).toBeNull();
    expect(entry?.dedicatedMemoryKb).toBeNull();
    expect(entry?.source).toBeNull();
    expect(buffers.currentGpuIds).toEqual([]);
  });

  it("does not open a gap for an absent adapter that never had usage", () => {
    const buffers = createLiveBuffers();
    pushSample(buffers, liveSample({ gpus: [liveGpu("gpu-a")] }), 0);

    pushSample(buffers, liveSample({ gpus: [] }), 0);

    expect(buffers.gpus.get(gpuA)?.usage.size).toBe(0);
  });

  it("retires an adapter on its third consecutive absence and reports it", () => {
    const buffers = createLiveBuffers();
    pushSample(
      buffers,
      liveSample({
        gpus: [liveGpu("gpu-a", { gpuUsage: 40 }), liveGpu("gpu-b")],
      }),
      0,
    );

    const retired = [1, 2, 3].map(
      () =>
        pushSample(buffers, liveSample({ gpus: [liveGpu("gpu-b")] }), 0)
          .retiredGpuIds,
    );

    expect(retired).toEqual([[], [], [gpuA]]);
    expect(buffers.gpus.has(gpuA)).toBe(false);
    expect(buffers.gpus.has(gpuB)).toBe(true);
  });

  it("restarts the grace period when the adapter reports again", () => {
    const buffers = createLiveBuffers();
    const withBoth = liveSample({
      gpus: [liveGpu("gpu-a", { gpuUsage: 40 }), liveGpu("gpu-b")],
    });
    const withB = liveSample({ gpus: [liveGpu("gpu-b")] });

    pushSample(buffers, withBoth, 0);
    pushSample(buffers, withB, 0);
    pushSample(buffers, withB, 0);
    pushSample(buffers, withBoth, 0);
    const first = pushSample(buffers, withB, 0);
    const second = pushSample(buffers, withB, 0);

    expect(first.retiredGpuIds).toEqual([]);
    expect(second.retiredGpuIds).toEqual([]);
    expect(buffers.gpus.has(gpuA)).toBe(true);
  });

  it("orders adapters by when their usage history began", () => {
    const buffers = createLiveBuffers();

    pushSample(
      buffers,
      liveSample({
        gpus: [liveGpu("gpu-a"), liveGpu("gpu-b", { gpuUsage: 5 })],
      }),
      0,
    );
    pushSample(
      buffers,
      liveSample({
        gpus: [
          liveGpu("gpu-a", { gpuUsage: 9 }),
          liveGpu("gpu-b", { gpuUsage: 6 }),
        ],
      }),
      0,
    );

    const withHistory = [...buffers.gpus]
      .filter(([, entry]) => entry.usage.size > 0)
      .map(([id]) => id);
    expect(withHistory).toEqual([gpuB, gpuA]);
  });
});

describe("pushSample power draw", () => {
  it("records nothing until a reading arrives", () => {
    const buffers = createLiveBuffers();

    pushSample(buffers, liveSample(), 0);

    expect(buffers.power.history.cpuWatts.size).toBe(0);
    expect(buffers.power.current.cpuWatts).toBeNull();
  });

  it("keeps the history going through empty samples once it has started", () => {
    const buffers = createLiveBuffers();
    pushSample(buffers, liveSample({ cpuPowerWatts: 10 }), 0);

    pushSample(buffers, liveSample(), 0);

    expect(buffers.power.history.cpuWatts.toArray()).toEqual([10, null]);
    expect(buffers.power.history.gpuWatts.toArray()).toEqual([null, null]);
  });

  it("spans missed seconds with nulls before the new reading, for power only", () => {
    const buffers = createLiveBuffers();
    pushSample(buffers, liveSample({ cpuUsage: 1, cpuPowerWatts: 10 }), 0);

    pushSample(buffers, liveSample({ cpuUsage: 2, cpuPowerWatts: 12 }), 3);

    expect(buffers.power.history.cpuWatts.toArray()).toEqual([
      10,
      null,
      null,
      null,
      12,
    ]);
    expect(buffers.cpu.toArray()).toEqual([1, 2]);
  });

  it("never grows the window past its length, however long the gap", () => {
    const buffers = createLiveBuffers();
    pushSample(buffers, liveSample({ cpuPowerWatts: 10 }), 0);

    pushSample(buffers, liveSample({ cpuPowerWatts: 12 }), windowLength - 1);

    expect(buffers.power.history.cpuWatts.size).toBe(windowLength);
    expect(buffers.power.history.cpuWatts.latest()).toBe(12);
    expect(buffers.power.history.cpuWatts.oldest()).toBeNull();
  });
});
