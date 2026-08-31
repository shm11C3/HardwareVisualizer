import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { LoadBandDumbbellRow } from "../utils/loadBandDumbbell";
import { LoadBandDumbbellChart } from "./LoadBandDumbbellChart";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const BASELINE_TITLE =
  "pages.insights.cooling.loadBandComparison.legend.baseline";
const RECENT_TITLE = "pages.insights.cooling.loadBandComparison.legend.recent";

const dotLeft = (title: string): number => {
  const dot = screen.getByTitle(title);
  return Number.parseFloat(dot.style.left);
};

afterEach(cleanup);

describe("LoadBandDumbbellChart", () => {
  it("places a negative thermal delta rather than collapsing it to the track's left end", () => {
    // The ambient-adjusted variant (#2046) draws thermal deltas, and Core
    // does not clamp a ΔT at zero: a machine idling below the room it sits
    // in is a real observation. A domain clamped at zero would pin the
    // -2 degC endpoint to 0% and make it indistinguishable from the
    // coldest reading of a window that never went negative.
    const rows: LoadBandDumbbellRow[] = [
      { band: "idle", comparable: true, baseline: -2, recent: 3, delta: 5 },
    ];

    render(<LoadBandDumbbellChart rows={rows} temperatureUnit="C" />);

    // Domain [-4, 5]: the baseline sits 2 of 9 degrees along the track.
    expect(dotLeft(`${BASELINE_TITLE}: -2.0°C`)).toBeCloseTo(200 / 9, 5);
    expect(dotLeft(`${RECENT_TITLE}: 3.0°C`)).toBeCloseTo(700 / 9, 5);
  });

  it("keeps an all-positive comparison positioned exactly as before", () => {
    // The absolute comparison shares this chart, so switching to a signed
    // domain must not move a CPU-temperature dumbbell by a pixel.
    const rows: LoadBandDumbbellRow[] = [
      {
        band: "idle",
        comparable: true,
        baseline: 32,
        recent: 33.5,
        delta: 1.5,
      },
    ];

    render(<LoadBandDumbbellChart rows={rows} temperatureUnit="C" />);

    // Domain [30, 36] on both the clamped and the signed derivation.
    expect(dotLeft(`${BASELINE_TITLE}: 32.0°C`)).toBeCloseTo(200 / 6, 5);
    expect(dotLeft(`${RECENT_TITLE}: 33.5°C`)).toBeCloseTo(350 / 6, 5);
  });

  it("reports a band Core could not compare instead of drawing a track", () => {
    render(
      <LoadBandDumbbellChart
        rows={[{ band: "high", comparable: false }]}
        temperatureUnit="C"
      />,
    );

    expect(
      screen.getByText(
        "pages.insights.cooling.loadBandComparison.notComparable",
      ),
    ).toBeInTheDocument();
  });
});
