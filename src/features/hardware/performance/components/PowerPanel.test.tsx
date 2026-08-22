import { cleanup, render, screen } from "@testing-library/react";
import { createStore, Provider } from "jotai";
import { afterEach, describe, expect, it, vi } from "vitest";
import { powerDrawAtom } from "@/features/hardware/store/chart";
import { PowerPanel } from "./PowerPanel";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("PowerPanel", () => {
  afterEach(cleanup);

  it("shows every component while preserving missing readings", () => {
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
    expect(screen.getByText("2.2 W")).toBeVisible();
    expect(screen.getAllByText("—")).toHaveLength(2);
  });
});
