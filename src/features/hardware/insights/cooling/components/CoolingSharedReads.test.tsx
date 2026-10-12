import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  within,
} from "@testing-library/react";
import { createStore, Provider } from "jotai";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  buildCoolingDailyTrendFixture,
  buildCoolingFanTrendFixture,
  coolingBaselineDeltaFixture,
} from "@/e2e/fixtures/cooling";
import {
  coolingInsightPeriodAtom,
  coolingInsightPeriodHydratedAtom,
} from "@/features/hardware/store/coolingPeriod";
import type {
  CoolingBaselineDelta,
  CoolingDailyTrendPoint,
  CoolingFanTrend,
} from "@/rspc/bindings";
import { CoverageStrip } from "./CoverageStrip";
import { ObservationStrip } from "./ObservationStrip";
import { ThermalTimelineLane } from "./ThermalTimelineLane";

const mocks = vi.hoisted(() => ({
  getCoolingTrend: vi.fn(),
  getCoolingFanTrend: vi.fn(),
  getCoolingBaselineDelta: vi.fn(),
  /** The rows the timeline handed to its lanes on the latest render. */
  timelineRowKeys: [] as string[],
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

// The lanes are drawn by recharts; these tests are about which state the
// timeline panel is in, not about the drawing.
vi.mock("./TimelineLanes", () => ({
  TimelineLanes: ({ rows }: { rows: { key: string }[] }) => {
    mocks.timelineRowKeys = rows.map((row) => row.key);
    return <div data-testid="timeline-lanes-stub" />;
  },
}));

const ok = <T,>(data: T) => ({ status: "ok" as const, data });

const dailyTrend: CoolingDailyTrendPoint[] = buildCoolingDailyTrendFixture(90);
const fanTrend: CoolingFanTrend = {
  series: buildCoolingFanTrendFixture(90),
  archiveHasReadings: true,
};

const flush = async () => {
  await act(async () => {
    for (let i = 0; i < 6; i++) {
      await Promise.resolve();
    }
  });
};

const seededStore = (period: "90d" | "1y") => {
  const store = createStore();
  store.set(coolingInsightPeriodAtom, period);
  store.set(coolingInsightPeriodHydratedAtom, true);
  return store;
};

const Panels = () => (
  <>
    <ObservationStrip />
    <ThermalTimelineLane />
    <CoverageStrip />
  </>
);

const mount = (store: ReturnType<typeof createStore>, ui: ReactNode) =>
  render(<Provider store={store}>{ui}</Provider>);

// The read mechanism (params rule, superseded requests, leave/return lifetime)
// is proven in store/sharedRead.test.ts; this file only proves the real panels
// are wired to one shared atom and render its failure state.
describe("Cooling panels sharing one read", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.timelineRowKeys = [];
    vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.getCoolingTrend.mockResolvedValue(ok(dailyTrend));
    mocks.getCoolingFanTrend.mockResolvedValue(ok(fanTrend));
    mocks.getCoolingBaselineDelta.mockResolvedValue(
      ok<CoolingBaselineDelta>(coolingBaselineDeltaFixture),
    );
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("feeds the timeline and the coverage strip from one daily-trend request", async () => {
    mount(seededStore("90d"), <Panels />);
    await flush();

    expect(screen.getByTestId("timeline-lanes-stub")).toBeInTheDocument();
    // The lanes draw the rows derived from that one daily-trend read: one
    // row per calendar day of the 90-day window (gaps included), so every
    // point the read returned is a row keyed by its date.
    expect(mocks.timelineRowKeys).toHaveLength(90);
    expect(mocks.timelineRowKeys).toEqual(
      expect.arrayContaining(dailyTrend.map((point) => point.date)),
    );
    expect(screen.getByRole("img")).toBeInTheDocument();
    expect(mocks.getCoolingTrend).toHaveBeenCalledTimes(1);
    expect(mocks.getCoolingTrend).toHaveBeenCalledWith(90);
    expect(mocks.getCoolingFanTrend).toHaveBeenCalledTimes(1);
    // The observation strip and the timeline's baseline band share one read.
    expect(mocks.getCoolingBaselineDelta).toHaveBeenCalledTimes(1);
  });

  it("renders the failure state in both panels and retries from either", async () => {
    mocks.getCoolingTrend.mockResolvedValue({
      status: "error" as const,
      error: "db locked",
    });
    mount(seededStore("90d"), <Panels />);
    await flush();

    const timeline = screen.getByTestId("cooling-thermal-timeline-lane");
    const strip = screen.getByTestId("cooling-coverage-strip");
    expect(within(timeline).getByTestId("load-failure")).toBeInTheDocument();
    expect(within(strip).getByTestId("load-failure")).toBeInTheDocument();
    expect(
      within(timeline).getByText("pages.insights.cooling.timeline.loadFailed"),
    ).toBeInTheDocument();
    expect(mocks.getCoolingTrend).toHaveBeenCalledTimes(1);

    mocks.getCoolingTrend.mockResolvedValue(ok(dailyTrend));
    fireEvent.click(
      within(strip).getByRole("button", { name: "shared.retry" }),
    );
    await flush();

    // One retry recovers every panel reading the same rollup.
    expect(mocks.getCoolingTrend).toHaveBeenCalledTimes(2);
    expect(screen.queryByTestId("load-failure")).toBeNull();
    expect(screen.getByTestId("timeline-lanes-stub")).toBeInTheDocument();
  });
});
