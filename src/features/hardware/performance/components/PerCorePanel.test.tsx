import { cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, describe, expect, it, vi } from "vitest";
import { processorsUsageHistoryAtom } from "@/features/hardware/store/chart";
import { PerCorePanel } from "./PerCorePanel";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: { lineGraphColor: { cpu: "75, 192, 192" } },
  }),
}));

describe("PerCorePanel", () => {
  afterEach(cleanup);

  it("waits for a sample before claiming per-core usage is unavailable", () => {
    const store = createStore();

    render(
      <Provider store={store}>
        <PerCorePanel />
      </Provider>,
    );

    expect(screen.getByTestId("per-core-loading")).toBeVisible();
    expect(
      screen.queryByText("pages.performance.perCoreUnavailable"),
    ).toBeNull();
  });

  it("reports absence once a sample arrived without per-core data", () => {
    const store = createStore();
    store.set(processorsUsageHistoryAtom, [[]]);

    render(
      <Provider store={store}>
        <PerCorePanel />
      </Provider>,
    );

    expect(
      screen.getByText("pages.performance.perCoreUnavailable"),
    ).toBeVisible();
  });

  it("renders one bar per logical processor", () => {
    const store = createStore();
    store.set(processorsUsageHistoryAtom, [[10, 55, 90, 33]]);

    render(
      <Provider store={store}>
        <PerCorePanel />
      </Provider>,
    );

    expect(screen.getByText("P0")).toBeVisible();
    expect(screen.getByText("P3")).toBeVisible();
    expect(screen.getByText("55%")).toBeVisible();
  });
});
