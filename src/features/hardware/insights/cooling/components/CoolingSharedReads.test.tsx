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
  TimelineLanes: () => <div data-testid="timeline-lanes-stub" />,
}));

type Deferred<T> = { promise: Promise<T>; resolve: (value: T) => void };

const defer = <T,>(): Deferred<T> => {
  let resolve: (value: T) => void = () => {};
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
};

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

describe("Cooling panels sharing one read", () => {
  beforeEach(() => {
    vi.clearAllMocks();
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
    expect(screen.getByRole("img")).toBeInTheDocument();
    expect(mocks.getCoolingTrend).toHaveBeenCalledTimes(1);
    expect(mocks.getCoolingTrend).toHaveBeenCalledWith(90);
    expect(mocks.getCoolingFanTrend).toHaveBeenCalledTimes(1);
    // The observation strip and the timeline's baseline band share one read.
    expect(mocks.getCoolingBaselineDelta).toHaveBeenCalledTimes(1);
  });

  it("shows the loading state, not the previous period's data, after a period change", async () => {
    const store = seededStore("90d");
    mount(store, <Panels />);
    await flush();
    expect(screen.getByTestId("timeline-lanes-stub")).toBeInTheDocument();

    const next = defer<ReturnType<typeof ok<CoolingDailyTrendPoint[]>>>();
    mocks.getCoolingTrend.mockReturnValue(next.promise);
    act(() => {
      store.set(coolingInsightPeriodAtom, "1y");
    });
    await flush();

    expect(mocks.getCoolingTrend).toHaveBeenLastCalledWith(365);
    expect(screen.getByTestId("cooling-timeline-loading")).toBeInTheDocument();
    expect(screen.queryByTestId("timeline-lanes-stub")).toBeNull();
    expect(
      screen.getByTestId("cooling-coverage-strip-loading"),
    ).toBeInTheDocument();
    expect(screen.queryByRole("img")).toBeNull();

    await act(async () => {
      next.resolve(ok(buildCoolingDailyTrendFixture(365)));
    });
    await flush();

    expect(screen.getByTestId("timeline-lanes-stub")).toBeInTheDocument();
    expect(screen.getByRole("img")).toBeInTheDocument();
  });

  it("does not duplicate an in-flight request when the tab is left and re-entered", async () => {
    const store = seededStore("90d");
    const trend = defer<ReturnType<typeof ok<CoolingDailyTrendPoint[]>>>();
    const fan = defer<ReturnType<typeof ok<CoolingFanTrend>>>();
    const baseline = defer<ReturnType<typeof ok<CoolingBaselineDelta>>>();
    mocks.getCoolingTrend.mockReturnValue(trend.promise);
    mocks.getCoolingFanTrend.mockReturnValue(fan.promise);
    mocks.getCoolingBaselineDelta.mockReturnValue(baseline.promise);

    const first = mount(store, <Panels />);
    await flush();
    first.unmount();
    mount(store, <Panels />);
    await flush();

    expect(mocks.getCoolingTrend).toHaveBeenCalledTimes(1);
    expect(mocks.getCoolingFanTrend).toHaveBeenCalledTimes(1);
    expect(mocks.getCoolingBaselineDelta).toHaveBeenCalledTimes(1);

    await act(async () => {
      trend.resolve(ok(dailyTrend));
      fan.resolve(ok(fanTrend));
      baseline.resolve(ok(coolingBaselineDeltaFixture));
    });
    await flush();

    expect(screen.getByTestId("timeline-lanes-stub")).toBeInTheDocument();
    expect(mocks.getCoolingTrend).toHaveBeenCalledTimes(1);
    expect(mocks.getCoolingFanTrend).toHaveBeenCalledTimes(1);
    expect(mocks.getCoolingBaselineDelta).toHaveBeenCalledTimes(1);
  });

  it("refetches once on return to a tab whose reads had settled, showing them meanwhile", async () => {
    const store = seededStore("90d");
    const first = mount(store, <Panels />);
    await flush();
    first.unmount();

    const refetch = defer<ReturnType<typeof ok<CoolingDailyTrendPoint[]>>>();
    mocks.getCoolingTrend.mockReturnValue(refetch.promise);
    mount(store, <Panels />);
    await flush();

    expect(mocks.getCoolingTrend).toHaveBeenCalledTimes(2);
    expect(screen.getByTestId("timeline-lanes-stub")).toBeInTheDocument();
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
