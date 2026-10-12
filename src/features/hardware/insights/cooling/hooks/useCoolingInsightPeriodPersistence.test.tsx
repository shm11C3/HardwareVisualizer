import { act, renderHook } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  COOLING_INSIGHT_PERIOD_STORE_KEY,
  useCoolingInsightPeriodPersistence,
} from "@/features/hardware/insights/cooling/hooks/useCoolingInsightPeriodPersistence";
import {
  coolingInsightPeriodAtom,
  coolingInsightPeriodHydratedAtom,
} from "@/features/hardware/store/coolingPeriod";

const mocks = vi.hoisted(() => ({
  entries: new Map<string, unknown>(),
  has: vi.fn(),
  get: vi.fn(),
  set: vi.fn(),
  save: vi.fn(),
}));

vi.mock("@/lib/tauriStore", () => ({
  getStoreInstance: async () => ({
    has: mocks.has,
    get: mocks.get,
    set: mocks.set,
    save: mocks.save,
  }),
}));

const setup = () => {
  const store = createStore();
  const wrapper = ({ children }: { children: ReactNode }) => (
    <Provider store={store}>{children}</Provider>
  );
  const hook = renderHook(() => useCoolingInsightPeriodPersistence(), {
    wrapper,
  });
  return { store, ...hook };
};

const settle = async () => {
  await act(async () => {
    for (let i = 0; i < 10; i++) {
      await Promise.resolve();
    }
  });
};

describe("useCoolingInsightPeriodPersistence", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.entries.clear();
    mocks.has.mockImplementation(async (key: string) => mocks.entries.has(key));
    mocks.get.mockImplementation(async (key: string) => mocks.entries.get(key));
    mocks.set.mockImplementation(async (key: string, value: unknown) => {
      mocks.entries.set(key, value);
    });
    mocks.save.mockResolvedValue(undefined);
  });

  it("stays unhydrated until the stored period has been read, then restores it", async () => {
    mocks.entries.set(COOLING_INSIGHT_PERIOD_STORE_KEY, "30d");
    const { store } = setup();

    expect(store.get(coolingInsightPeriodHydratedAtom)).toBe(false);

    await settle();

    expect(store.get(coolingInsightPeriodHydratedAtom)).toBe(true);
    expect(store.get(coolingInsightPeriodAtom)).toBe("30d");
    // Restoring is not a change: nothing is written back.
    expect(mocks.set).not.toHaveBeenCalled();
  });

  it("settles on the default when nothing is stored yet", async () => {
    const { store } = setup();
    await settle();

    expect(store.get(coolingInsightPeriodHydratedAtom)).toBe(true);
    expect(store.get(coolingInsightPeriodAtom)).toBe("24h");
  });

  it("writes a changed selection to the store", async () => {
    mocks.entries.set(COOLING_INSIGHT_PERIOD_STORE_KEY, "24h");
    const { store } = setup();
    await settle();

    act(() => {
      store.set(coolingInsightPeriodAtom, "1y");
    });
    await settle();

    expect(mocks.set).toHaveBeenCalledWith(
      COOLING_INSIGHT_PERIOD_STORE_KEY,
      "1y",
    );
    expect(mocks.entries.get(COOLING_INSIGHT_PERIOD_STORE_KEY)).toBe("1y");
  });

  it("settles on the default when the stored value cannot be read, and still persists a later choice", async () => {
    mocks.has.mockRejectedValue(new Error("store unavailable"));
    const { store } = setup();
    await settle();

    expect(store.get(coolingInsightPeriodHydratedAtom)).toBe(true);
    expect(store.get(coolingInsightPeriodAtom)).toBe("24h");
    expect(mocks.set).not.toHaveBeenCalled();

    act(() => {
      store.set(coolingInsightPeriodAtom, "7d");
    });
    await settle();

    expect(mocks.set).toHaveBeenCalledWith(
      COOLING_INSIGHT_PERIOD_STORE_KEY,
      "7d",
    );
  });

  it("does not overwrite a selection already made when a later visit re-reads the store", async () => {
    mocks.entries.set(COOLING_INSIGHT_PERIOD_STORE_KEY, "30d");
    const store = createStore();
    store.set(coolingInsightPeriodAtom, "7d");
    store.set(coolingInsightPeriodHydratedAtom, true);

    renderHook(() => useCoolingInsightPeriodPersistence(), {
      wrapper: ({ children }: { children: ReactNode }) => (
        <Provider store={store}>{children}</Provider>
      ),
    });
    await settle();

    expect(store.get(coolingInsightPeriodAtom)).toBe("7d");
    expect(mocks.set).toHaveBeenCalledWith(
      COOLING_INSIGHT_PERIOD_STORE_KEY,
      "7d",
    );
  });
});
