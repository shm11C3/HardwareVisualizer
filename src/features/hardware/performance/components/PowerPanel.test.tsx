import { cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, describe, expect, it, vi } from "vitest";
import { liveSample } from "@/features/hardware/live/liveSamples.testHelpers";
import { publishLiveSampleAtom } from "@/features/hardware/store/liveMetrics";
import { PowerPanel } from "./PowerPanel";

let powerDisplayTargets = ["cpu", "gpu", "package"];

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: { powerDisplayTargets } }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("PowerPanel", () => {
  afterEach(cleanup);

  it("shows only selected components while preserving missing readings", () => {
    powerDisplayTargets = ["cpu", "ane", "package"];
    const store = createStore();
    store.set(
      publishLiveSampleAtom,
      liveSample({
        cpuPowerWatts: 10.1,
        gpuPowerWatts: 2.2,
        anePowerWatts: null,
        packagePowerWatts: null,
      }),
      0,
    );

    render(
      <Provider store={store}>
        <PowerPanel />
      </Provider>,
    );

    expect(screen.getByText("10.1 W")).toBeVisible();
    expect(screen.queryByText("2.2 W")).toBeNull();
    expect(screen.getAllByText("—")).toHaveLength(2);
    expect(screen.getByText("pages.performance.power.ane")).toBeVisible();
    expect(screen.queryByText("pages.performance.power.gpu")).toBeNull();
  });
});
