import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { coolingLoadTemperatureExplorerEstablishingFixture } from "@/e2e/fixtures/cooling";
import { LoadTemperatureExplorerPanel } from "./LoadTemperatureExplorerPanel";

const mocks = vi.hoisted(() => ({
  getCoolingLoadTemperatureExplorer: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
    i18n: { language: "en" },
  }),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: { temperatureUnit: "C" } }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: mocks,
}));

/**
 * The Explorer keeps its own hook rather than a shared atom: only this panel
 * renders it, and its collapsed state is what decides whether it asks at
 * all (DP-04).
 */
describe("LoadTemperatureExplorerPanel fetch gate", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    mocks.getCoolingLoadTemperatureExplorer.mockResolvedValue({
      status: "ok",
      data: coolingLoadTemperatureExplorerEstablishingFixture,
    });
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("issues no request while collapsed, however long the panel stays mounted", async () => {
    render(<LoadTemperatureExplorerPanel />);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(5 * 60_000);
    });

    expect(mocks.getCoolingLoadTemperatureExplorer).not.toHaveBeenCalled();
  });

  it("asks once when expanded and stops asking once folded away again", async () => {
    render(<LoadTemperatureExplorerPanel />);

    fireEvent.click(screen.getByTestId("cooling-explorer-trigger"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });

    expect(mocks.getCoolingLoadTemperatureExplorer).toHaveBeenCalledTimes(1);
    expect(mocks.getCoolingLoadTemperatureExplorer).toHaveBeenCalledWith(28);

    // Folding it away again stops asking; no timer revives the query.
    fireEvent.click(screen.getByTestId("cooling-explorer-trigger"));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5 * 60_000);
    });

    expect(mocks.getCoolingLoadTemperatureExplorer).toHaveBeenCalledTimes(1);
  });
});
