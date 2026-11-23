import { act, render } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { describe, expect, it, vi } from "vitest";
import { CoolingInsightView } from "@/features/hardware/insights/cooling/CoolingInsightView";
import { coolingInsightPeriodHydratedAtom } from "@/features/hardware/store/coolingPeriod";

// Hydration is covered by the persistence hook's own test; here the store
// stays as the test seeds it.
vi.mock(
  "@/features/hardware/insights/cooling/hooks/useCoolingInsightPeriodPersistence",
  () => ({ useCoolingInsightPeriodPersistence: () => undefined }),
);

const panels = [
  "CoolingPeriodSelect",
  "ObservationStrip",
  "ThermalTimelineLane",
  "SensorStatusNote",
  "CoverageStrip",
  "LoadBandComparisonPanel",
  "CovariateComparisonPanel",
  "LoadTemperatureExplorerPanel",
] as const;

// Every panel is a stub that records the props it was given.
const received = vi.hoisted(() => new Map<string, object>());

vi.mock("./components/CoolingPeriodSelect", () => ({
  CoolingPeriodSelect: (props: object) => {
    received.set("CoolingPeriodSelect", props);
    return <div data-testid="CoolingPeriodSelect" />;
  },
}));
vi.mock("./components/ObservationStrip", () => ({
  ObservationStrip: (props: object) => {
    received.set("ObservationStrip", props);
    return <div data-testid="ObservationStrip" />;
  },
}));
vi.mock("./components/ThermalTimelineLane", () => ({
  ThermalTimelineLane: (props: object) => {
    received.set("ThermalTimelineLane", props);
    return <div data-testid="ThermalTimelineLane" />;
  },
}));
vi.mock("./components/SensorStatusNote", () => ({
  SensorStatusNote: (props: object) => {
    received.set("SensorStatusNote", props);
    return <div data-testid="SensorStatusNote" />;
  },
}));
vi.mock("./components/CoverageStrip", () => ({
  CoverageStrip: (props: object) => {
    received.set("CoverageStrip", props);
    return <div data-testid="CoverageStrip" />;
  },
}));
vi.mock("./components/LoadBandComparisonPanel", () => ({
  LoadBandComparisonPanel: (props: object) => {
    received.set("LoadBandComparisonPanel", props);
    return <div data-testid="LoadBandComparisonPanel" />;
  },
}));
vi.mock("./components/CovariateComparisonPanel", () => ({
  CovariateComparisonPanel: (props: object) => {
    received.set("CovariateComparisonPanel", props);
    return <div data-testid="CovariateComparisonPanel" />;
  },
}));
vi.mock("./components/LoadTemperatureExplorerPanel", () => ({
  LoadTemperatureExplorerPanel: (props: object) => {
    received.set("LoadTemperatureExplorerPanel", props);
    return <div data-testid="LoadTemperatureExplorerPanel" />;
  },
}));

describe("CoolingInsightView", () => {
  it("mounts no panel until the stored period has been restored", () => {
    const store = createStore();
    const { container, queryByTestId } = render(
      <Provider store={store}>
        <CoolingInsightView />
      </Provider>,
    );

    expect(container).toBeEmptyDOMElement();
    expect(queryByTestId("ObservationStrip")).toBeNull();

    act(() => {
      store.set(coolingInsightPeriodHydratedAtom, true);
    });

    for (const panel of panels) {
      expect(container.querySelector(`[data-testid="${panel}"]`)).not.toBe(
        null,
      );
    }
  });

  it("passes no read results, retries or periods to its panels", () => {
    const store = createStore();
    store.set(coolingInsightPeriodHydratedAtom, true);
    render(
      <Provider store={store}>
        <CoolingInsightView />
      </Provider>,
    );

    for (const panel of panels) {
      expect(received.get(panel)).toEqual({});
    }
  });
});
