import { cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  cpuUsageHistoryAtom,
  motherboardTempsAtom,
} from "@/features/hardware/store/chart";
import { MotherboardSensorsPanel } from "./MotherboardSensorsPanel";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: { temperatureUnit: "C" } }),
}));

describe("MotherboardSensorsPanel", () => {
  afterEach(cleanup);

  it("waits for a sample before claiming the platform has no sensors", () => {
    const store = createStore();

    render(
      <Provider store={store}>
        <MotherboardSensorsPanel />
      </Provider>,
    );

    // Before the first hardware update, empty atoms mean "not yet", not "none".
    expect(screen.getByTestId("motherboard-sensors-loading")).toBeVisible();
    expect(
      screen.queryByText("pages.performance.motherboardSensorsUnavailable"),
    ).toBeNull();
  });

  it("reports absence once a sample arrived without sensor readings", () => {
    const store = createStore();
    store.set(cpuUsageHistoryAtom, [42]);

    render(
      <Provider store={store}>
        <MotherboardSensorsPanel />
      </Provider>,
    );

    expect(
      screen.getByText("pages.performance.motherboardSensorsUnavailable"),
    ).toBeVisible();
    expect(screen.queryByTestId("motherboard-sensors-loading")).toBeNull();
  });

  it("renders readings as soon as sensors report, regardless of sample state", () => {
    const store = createStore();
    store.set(motherboardTempsAtom, [
      { name: "SYSTIN", value: 34, source: "NCT6799D" },
    ]);

    render(
      <Provider store={store}>
        <MotherboardSensorsPanel />
      </Provider>,
    );

    expect(screen.getByText("SYSTIN")).toBeVisible();
    expect(screen.getByText("34 °C")).toBeVisible();
  });
});
