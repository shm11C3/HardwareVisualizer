import { act, renderHook } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useCoolingArchiveRefresh } from "@/features/hardware/insights/cooling/hooks/useCoolingArchiveRefresh";
import {
  coolingInsightPeriodAtom,
  coolingInsightPeriodHydratedAtom,
} from "@/features/hardware/store/coolingPeriod";
import { coolingArchiveAtom } from "@/features/hardware/store/coolingReads";

const mocks = vi.hoisted(() => ({
  getDataArchiveSeries: vi.fn(),
  getFanArchiveSeries: vi.fn(),
  getAmbientArchiveSeries: vi.fn(),
  getCoolingTrend: vi.fn(),
}));

vi.mock("@/rspc/bindings", () => ({ commands: mocks }));

const MINUTE = 60_000;

/** One archive timeline fetch asks for ambient exactly once. */
const archiveFetches = () => mocks.getAmbientArchiveSeries.mock.calls.length;

const setup = (period: "24h" | "7d" | "90d") => {
  const store = createStore();
  store.set(coolingInsightPeriodAtom, period);
  store.set(coolingInsightPeriodHydratedAtom, true);
  // The timeline panel is what subscribes; the hook only drives refreshes.
  const unsubscribe = store.sub(coolingArchiveAtom, () => {});
  const wrapper = ({ children }: { children: ReactNode }) => (
    <Provider store={store}>{children}</Provider>
  );
  const hook = renderHook(() => useCoolingArchiveRefresh(), { wrapper });
  return { store, unsubscribe, ...hook };
};

describe("useCoolingArchiveRefresh", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    mocks.getDataArchiveSeries.mockResolvedValue({ status: "ok", data: [] });
    mocks.getFanArchiveSeries.mockResolvedValue({ status: "ok", data: [] });
    mocks.getAmbientArchiveSeries.mockResolvedValue({
      status: "ok",
      data: { sources: [], buckets: [] },
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    Reflect.deleteProperty(document, "hidden");
  });

  it("re-reads the archive every 60 seconds while mounted", async () => {
    setup("7d");
    await vi.advanceTimersByTimeAsync(0);
    expect(archiveFetches()).toBe(1);

    await vi.advanceTimersByTimeAsync(MINUTE - 1);
    expect(archiveFetches()).toBe(1);
    await vi.advanceTimersByTimeAsync(1);
    expect(archiveFetches()).toBe(2);
    await vi.advanceTimersByTimeAsync(MINUTE);
    expect(archiveFetches()).toBe(3);
  });

  it("stops refreshing when the panel unmounts", async () => {
    const { unmount } = setup("7d");
    await vi.advanceTimersByTimeAsync(MINUTE);
    expect(archiveFetches()).toBe(2);

    unmount();
    await vi.advanceTimersByTimeAsync(5 * MINUTE);

    expect(archiveFetches()).toBe(2);
  });

  it("fetches nothing while no panel is subscribed, even with the hook mounted", async () => {
    const { unsubscribe } = setup("7d");
    await vi.advanceTimersByTimeAsync(0);
    unsubscribe();
    const before = archiveFetches();

    await vi.advanceTimersByTimeAsync(5 * MINUTE);

    expect(archiveFetches()).toBe(before);
  });

  it("keeps refreshing while the document is hidden, as the read it replaced did", async () => {
    Object.defineProperty(document, "hidden", {
      configurable: true,
      value: true,
    });
    setup("7d");
    await vi.advanceTimersByTimeAsync(MINUTE);

    expect(archiveFetches()).toBe(2);
  });

  it("does not run for the daily-rollup periods", async () => {
    setup("90d");
    await vi.advanceTimersByTimeAsync(5 * MINUTE);

    expect(archiveFetches()).toBe(0);
    expect(mocks.getDataArchiveSeries).not.toHaveBeenCalled();
  });

  it("restarts the interval when the period changes, since that change already fetches", async () => {
    const { store } = setup("7d");
    await vi.advanceTimersByTimeAsync(30_000);

    act(() => {
      store.set(coolingInsightPeriodAtom, "24h");
    });
    await vi.advanceTimersByTimeAsync(0);
    expect(archiveFetches()).toBe(2);

    await vi.advanceTimersByTimeAsync(MINUTE - 1);
    expect(archiveFetches()).toBe(2);
    await vi.advanceTimersByTimeAsync(1);
    expect(archiveFetches()).toBe(3);
  });
});
