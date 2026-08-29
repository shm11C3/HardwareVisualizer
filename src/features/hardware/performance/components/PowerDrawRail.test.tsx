import { render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { describe, expect, it, vi } from "vitest";
import { powerDrawAtom } from "@/features/hardware/store/chart";
import { PowerDrawRail } from "./PowerDrawRail";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: { powerDisplayTargets: ["cpu", "ane", "package"] },
  }),
}));

describe("PowerDrawRail", () => {
  it("shows selected current readings and preserves unavailable values", () => {
    const store = createStore();
    store.set(powerDrawAtom, {
      cpuWatts: 10.1,
      gpuWatts: 2.2,
      aneWatts: null,
      packageWatts: 12.3,
    });

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
