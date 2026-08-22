import { cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, describe, expect, it, vi } from "vitest";
import { powerDrawAtom } from "@/features/hardware/store/chart";
import { PowerPanel } from "./PowerPanel";

let powerDisplayTargets = ["cpu", "gpu", "package"];

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
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
    store.set(powerDrawAtom, {
      cpuWatts: 10.1,
      gpuWatts: 2.2,
      aneWatts: null,
      packageWatts: null,
    });

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
