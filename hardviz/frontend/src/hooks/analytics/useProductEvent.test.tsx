import { renderHook } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ProductAnalyticsClientSettings } from "@/rspc/bindings";
import { settingsAtom } from "@/store/settings";
import { useProductEvent } from "./useProductEvent";

const mocks = vi.hoisted(() => ({ recordProductEvent: vi.fn() }));

vi.mock("@/rspc/bindings", () => ({
  commands: { recordProductEvent: mocks.recordProductEvent },
}));

const withConsent = (productAnalytics: ProductAnalyticsClientSettings) => {
  const store = createStore();
  store.set(settingsAtom, (prev) => ({ ...prev, productAnalytics }));
  const wrapper = ({ children }: { children: ReactNode }) => (
    <Provider store={store}>{children}</Provider>
  );
  return { store, wrapper };
};

const granted: ProductAnalyticsClientSettings = {
  consent: "granted",
  consentVersion: 1,
  currentConsentVersion: 1,
};

describe("useProductEvent", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.recordProductEvent.mockResolvedValue(undefined);
  });

  it.each<[string, ProductAnalyticsClientSettings]>([
    [
      "undecided",
      { consent: "undecided", consentVersion: 0, currentConsentVersion: 1 },
    ],
    [
      "declined",
      { consent: "declined", consentVersion: 1, currentConsentVersion: 1 },
    ],
    [
      "granted under an older contract",
      { consent: "granted", consentVersion: 1, currentConsentVersion: 2 },
    ],
  ])("makes no IPC call while consent is %s", async (_label, state) => {
    const { wrapper } = withConsent(state);
    const { result } = renderHook(() => useProductEvent(), { wrapper });

    await result.current({ screenView: "dashboard" });

    expect(mocks.recordProductEvent).not.toHaveBeenCalled();
  });

  it("sends the event once consent is effective", async () => {
    const { wrapper } = withConsent(granted);
    const { result } = renderHook(() => useProductEvent(), { wrapper });

    await result.current({ capability: "hardwareReportExported" });

    expect(mocks.recordProductEvent).toHaveBeenCalledWith({
      capability: "hardwareReportExported",
    });
  });

  it("reads consent at call time without rerendering or changing identity", async () => {
    const { store, wrapper } = withConsent({
      consent: "undecided",
      consentVersion: 0,
      currentConsentVersion: 1,
    });
    let renders = 0;
    const { result } = renderHook(
      () => {
        renders += 1;
        return useProductEvent();
      },
      { wrapper },
    );
    const record = result.current;
    const rendersBefore = renders;

    store.set(settingsAtom, (prev) => ({ ...prev, productAnalytics: granted }));
    await record({ screenView: "insights" });

    expect(renders).toBe(rendersBefore);
    expect(result.current).toBe(record);
    expect(mocks.recordProductEvent).toHaveBeenCalledWith({
      screenView: "insights",
    });
  });

  it("logs a failed call and never throws into the caller", async () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    mocks.recordProductEvent.mockRejectedValue(new Error("ipc down"));
    const { wrapper } = withConsent(granted);
    const { result } = renderHook(() => useProductEvent(), { wrapper });

    await expect(
      result.current({ screenView: "usage" }),
    ).resolves.toBeUndefined();

    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });
});
