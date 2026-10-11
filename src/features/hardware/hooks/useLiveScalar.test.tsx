import { act, renderHook } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { createElement } from "react";
import { describe, expect, it } from "vitest";
import { useLiveScalar } from "@/features/hardware/hooks/useLiveScalar";
import { liveSample } from "@/features/hardware/live/liveSamples.testHelpers";
import { publishLiveSampleAtom } from "@/features/hardware/store/liveMetrics";

const mount = (channel: Parameters<typeof useLiveScalar>[0]) => {
  const store = createStore();
  let renders = 0;
  const hook = renderHook(
    () => {
      renders += 1;
      return useLiveScalar(channel);
    },
    { wrapper: ({ children }) => createElement(Provider, { store }, children) },
  );
  const publish = (overrides: Parameters<typeof liveSample>[0]) =>
    act(() => store.set(publishLiveSampleAtom, liveSample(overrides), 0));
  return { hook, publish, renderCount: () => renders };
};

describe("useLiveScalar", () => {
  it("is null before the first sample, then the latest value", () => {
    const { hook, publish } = mount("cpu");
    expect(hook.result.current).toBeNull();

    publish({ cpuUsage: 33 });

    expect(hook.result.current).toBe(33);
  });

  it("reads the memory channel", () => {
    const { hook, publish } = mount("memory");

    publish({ cpuUsage: 1, memoryUsage: 64 });

    expect(hook.result.current).toBe(64);
  });

  it("re-renders only when the value changes", () => {
    const { publish, renderCount } = mount("cpu");
    publish({ cpuUsage: 33, memoryUsage: 1 });
    const before = renderCount();

    publish({ cpuUsage: 33, memoryUsage: 2 });
    publish({ cpuUsage: 33, memoryUsage: 3 });

    expect(renderCount()).toBe(before);

    publish({ cpuUsage: 34 });

    expect(renderCount()).toBe(before + 1);
  });
});
