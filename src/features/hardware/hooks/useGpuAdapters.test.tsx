import { act, renderHook } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { asLiveGpuId, liveGpuRecord } from "@/features/hardware/gpuIdentity";
import { useGpuAdapters } from "@/features/hardware/hooks/useGpuAdapters";
import {
  liveGpu,
  liveSample,
} from "@/features/hardware/live/liveSamples.testHelpers";
import { gpuNamesAtom } from "@/features/hardware/store/gpu";
import { publishLiveSampleAtom } from "@/features/hardware/store/liveMetrics";

const nvidia = asLiveGpuId("nvapi:1");
const intel = asLiveGpuId("pci:0:2:0");

const mount = () => {
  const store = createStore();
  store.set(
    gpuNamesAtom,
    liveGpuRecord([
      [nvidia, "NVIDIA GeForce RTX 4080"],
      [intel, "Intel UHD Graphics 770"],
    ]),
  );
  store.set(
    publishLiveSampleAtom,
    liveSample({ gpus: [liveGpu(nvidia, { gpuUsage: 70 })] }),
    0,
  );

  let renders = 0;
  const hook = renderHook(
    () => {
      renders += 1;
      return useGpuAdapters();
    },
    {
      wrapper: ({ children }) => createElement(Provider, { store }, children),
    },
  );
  return { store, hook, renderCount: () => renders };
};

describe("useGpuAdapters", () => {
  it("answers which adapters exist, which one is effective and whether it reports", () => {
    const { hook } = mount();

    expect(hook.result.current.adapters.map((adapter) => adapter.id)).toEqual([
      nvidia,
      intel,
    ]);
    expect(hook.result.current.effectiveGpuId).toBe(nvidia);
    expect(hook.result.current.effectiveAdapter?.id).toBe(nvidia);
    expect(hook.result.current.hasNoReadings).toBe(false);
  });

  it("follows an explicit selection, including one that reports nothing", () => {
    const { hook } = mount();

    act(() => hook.result.current.selectGpu(intel));

    expect(hook.result.current.selectedGpuId).toBe(intel);
    expect(hook.result.current.effectiveGpuId).toBe(intel);
    expect(hook.result.current.hasNoReadings).toBe(true);
  });

  it("does not re-render when only readings change", () => {
    const { store, hook, renderCount } = mount();
    const adapters = hook.result.current.adapters;
    const before = renderCount();

    act(() => {
      store.set(
        publishLiveSampleAtom,
        liveSample({
          gpus: [
            liveGpu(nvidia, {
              gpuName: "NVIDIA GeForce RTX 4080",
              gpuUsage: 71,
              gpuTemperature: 61,
            }),
          ],
        }),
        0,
      );
    });

    expect(renderCount()).toBe(before);
    expect(hook.result.current.adapters).toBe(adapters);
  });

  it("re-renders when an adapter appears", () => {
    const { store, hook, renderCount } = mount();
    const before = renderCount();

    act(() => {
      store.set(
        gpuNamesAtom,
        liveGpuRecord([
          [nvidia, "NVIDIA GeForce RTX 4080"],
          [intel, "Intel UHD Graphics 770"],
          [asLiveGpuId("pci:0:3:0"), "AMD Radeon"],
        ]),
      );
    });

    expect(renderCount()).toBeGreaterThan(before);
    expect(hook.result.current.adapters).toHaveLength(3);
  });
});
