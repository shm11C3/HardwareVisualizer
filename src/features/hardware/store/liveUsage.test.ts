import { createStore } from "jotai";
import { describe, expect, it, vi } from "vitest";
import { liveSample } from "@/features/hardware/live/liveSamples.testHelpers";
import { publishLiveSampleAtom } from "@/features/hardware/store/liveMetrics";
import {
  hasCpuUsageHistoryAtom,
  latestProcessorCountAtom,
  processorCountAtom,
} from "@/features/hardware/store/liveUsage";

const publish = (
  store: ReturnType<typeof createStore>,
  overrides: Parameters<typeof liveSample>[0],
) => store.set(publishLiveSampleAtom, liveSample(overrides), 0);

/**
 * The derived scalars exist so a reader that needs a count or a flag does not
 * re-render with the per-second series behind it (#1638).
 */
describe("live usage scalars", () => {
  it("count logical processors, and are zero before the first sample", () => {
    const store = createStore();
    expect(store.get(processorCountAtom)).toBe(0);
    expect(store.get(latestProcessorCountAtom)).toBe(0);

    publish(store, { processorsUsage: [10, 20, 30, 40] });

    expect(store.get(processorCountAtom)).toBe(4);
    expect(store.get(latestProcessorCountAtom)).toBe(4);
  });

  it("read the oldest and the newest sample respectively", () => {
    const store = createStore();
    publish(store, { processorsUsage: [1, 2] });
    publish(store, { processorsUsage: [1, 2, 3, 4] });
    publish(store, { processorsUsage: [] });

    expect(store.get(processorCountAtom)).toBe(2);
    expect(store.get(latestProcessorCountAtom)).toBe(0);
  });

  it("tell whether a CPU sample has arrived", () => {
    const store = createStore();
    expect(store.get(hasCpuUsageHistoryAtom)).toBe(false);

    publish(store, { cpuUsage: null });

    expect(store.get(hasCpuUsageHistoryAtom)).toBe(true);
  });

  it("do not notify subscribers while the series grows but the scalar holds", () => {
    const store = createStore();
    publish(store, { cpuUsage: 10, processorsUsage: [1, 2] });
    const onCount = vi.fn();
    const onLatest = vi.fn();
    const onHas = vi.fn();
    store.sub(processorCountAtom, onCount);
    store.sub(latestProcessorCountAtom, onLatest);
    store.sub(hasCpuUsageHistoryAtom, onHas);

    publish(store, { cpuUsage: 20, processorsUsage: [3, 4] });

    expect(onCount).not.toHaveBeenCalled();
    expect(onLatest).not.toHaveBeenCalled();
    expect(onHas).not.toHaveBeenCalled();
  });
});
