import { act, renderHook } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { asLiveGpuId } from "@/features/hardware/gpuIdentity";
import { useLiveSeries } from "@/features/hardware/hooks/useLiveSeries";
import {
  liveGpu,
  liveSample,
  paddedHistory,
} from "@/features/hardware/live/liveSamples.testHelpers";
import { publishLiveSampleAtom } from "@/features/hardware/store/liveMetrics";

const mount = (channel: Parameters<typeof useLiveSeries>[0]) => {
  const store = createStore();
  const publish = (overrides: Parameters<typeof liveSample>[0]) =>
    act(() => store.set(publishLiveSampleAtom, liveSample(overrides), 0));
  const hook = renderHook(() => useLiveSeries(channel), {
    wrapper: ({ children }) => createElement(Provider, { store }, children),
  });
  return { hook, publish };
};

describe("useLiveSeries", () => {
  it("reads the CPU window, padded, and follows each sample", () => {
    const { hook, publish } = mount({ kind: "cpu" });
    expect(hook.result.current).toEqual(paddedHistory());

    publish({ cpuUsage: 10 });
    publish({ cpuUsage: 20 });

    expect(hook.result.current).toEqual(paddedHistory(10, 20));
  });

  it("reads the memory window", () => {
    const { hook, publish } = mount({ kind: "memory" });

    publish({ memoryUsage: 61 });

    expect(hook.result.current).toEqual(paddedHistory(61));
  });

  it("reads one processor's window by index", () => {
    const { hook, publish } = mount({ kind: "processor", index: 1 });

    publish({ processorsUsage: [5, 6, 7] });

    expect(hook.result.current).toEqual(paddedHistory(6));
  });

  it("reads one adapter's window by id", () => {
    const { hook, publish } = mount({
      kind: "gpu",
      id: asLiveGpuId("gpu-b"),
    });

    publish({
      gpus: [
        liveGpu("gpu-a", { gpuUsage: 1 }),
        liveGpu("gpu-b", { gpuUsage: 2 }),
      ],
    });

    expect(hook.result.current).toEqual(paddedHistory(2));
  });
});
