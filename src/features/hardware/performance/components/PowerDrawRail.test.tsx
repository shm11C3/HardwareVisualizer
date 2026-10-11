import { render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { describe, expect, it, vi } from "vitest";
import { liveSample } from "@/features/hardware/live/liveSamples.testHelpers";
import { publishLiveSampleAtom } from "@/features/hardware/store/liveMetrics";
import { PowerDrawRail } from "./PowerDrawRail";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: { powerDisplayTargets: ["cpu", "ane", "package"] },
  }),
}));

describe("PowerDrawRail", () => {
  it("shows selected current readings and preserves unavailable values", () => {
    const store = createStore();
    store.set(
      publishLiveSampleAtom,
      liveSample({
        cpuPowerWatts: 10.1,
        gpuPowerWatts: 2.2,
        anePowerWatts: null,
        packagePowerWatts: 12.3,
      }),
      0,
    );

    render(
      <Provider store={store}>
        <PowerDrawRail />
      </Provider>,
    );

    const rail = screen.getByTestId("performance-monitor-power-rail");
    expect(rail).toHaveTextContent("pages.performance.power.package");
    expect(rail).toHaveTextContent("12.3 W");
    expect(rail).toHaveTextContent("pages.performance.power.cpu");
    expect(rail).toHaveTextContent("10.1 W");
    expect(rail).toHaveTextContent("pages.performance.power.ane");
    expect(rail).toHaveTextContent("—");
    expect(rail).not.toHaveTextContent("pages.performance.power.gpu");
  });
});
