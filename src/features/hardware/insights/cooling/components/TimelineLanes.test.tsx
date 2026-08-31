import { cleanup, render, screen, within } from "@testing-library/react";
import { cloneElement, isValidElement, type ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { AmbientLaneRow } from "../utils/ambientTimeline";
import { type FanSeries, resolveFanSeries } from "../utils/fanTimeline";
import type { ThermalTimelineRow } from "../utils/thermalTimeline";
import { TimelineLanes } from "./TimelineLanes";

const mocks = vi.hoisted(() => ({
  /** The row the mocked tooltip pretends the cursor is over. */
  hoveredRow: null as unknown,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/components/ui/chart", () => ({
  ChartContainer: ({
    children,
    ...props
  }: {
    children: ReactNode;
    "data-testid"?: string;
  }) => <div data-testid={props["data-testid"]}>{children}</div>,
  // Recharts only mounts tooltip content on hover, which jsdom cannot
  // drive. Render it eagerly with a fixed hovered row instead: what is
  // under test is *which lane* carries the content and what it reports,
  // not recharts' own pointer handling.
  ChartTooltip: ({ content }: { content?: unknown }) =>
    isValidElement(content)
      ? cloneElement(content as React.ReactElement<Record<string, unknown>>, {
          active: true,
          payload: [{ payload: mocks.hoveredRow }],
        })
      : null,
}));

vi.mock("recharts", () => ({
  ComposedChart: ({ children }: { children: ReactNode }) => <>{children}</>,
  Area: () => null,
  Bar: () => null,
  Line: () => null,
  CartesianGrid: () => null,
  ReferenceArea: () => null,
  ReferenceLine: () => null,
  XAxis: () => null,
  YAxis: () => null,
}));

const row = (overrides: Partial<ThermalTimelineRow>): ThermalTimelineRow => ({
  key: "0",
  label: "12:00",
  temperatureAvg: null,
  temperatureMin: null,
  temperatureMax: null,
  temperatureRange: null,
  idleTemperature: null,
  cpuUsage: null,
  loadIdle: null,
  loadLow: null,
  loadMid: null,
  loadHigh: null,
  powerAvg: null,
  powerMin: null,
  powerMax: null,
  powerRange: null,
  ...overrides,
});

type FanFixture = {
  domain: [number, number] | null;
  values: Record<string, number | null>;
  series: FanSeries[];
};

const NO_FAN: FanFixture = { domain: null, values: {}, series: [] };

const withFans = (values: Record<string, number | null>): FanFixture => ({
  domain: [0, 2000],
  values,
  series: resolveFanSeries(["Fan 1"]),
});

type AmbientFixture = {
  domain: [number, number] | null;
  row: Omit<AmbientLaneRow, "key" | "label">;
};

const NO_AMBIENT: AmbientFixture = {
  domain: null,
  row: { ambient: null, delta: null },
};

const withAmbient = (
  row: Omit<AmbientLaneRow, "key" | "label">,
): AmbientFixture => ({ domain: [18, 30], row });

const renderLanes = (
  hovered: ThermalTimelineRow,
  domain: [number, number] | null,
  powerDomain: [number, number] | null,
  fan: FanFixture = NO_FAN,
  ambient: AmbientFixture = NO_AMBIENT,
) => {
  mocks.hoveredRow = hovered;
  return render(
    <TimelineLanes
      rows={[hovered]}
      domain={domain}
      powerDomain={powerDomain}
      fanRows={[{ key: hovered.key, label: hovered.label, values: fan.values }]}
      fanSeries={fan.series}
      fanDomain={fan.domain}
      ambientRows={[{ key: hovered.key, label: hovered.label, ...ambient.row }]}
      ambientDomain={ambient.domain}
      baseline={null}
      loadMode="usage"
      temperatureUnit="C"
    />,
  );
};

afterEach(cleanup);

describe("TimelineLanes shared tooltip", () => {
  it("puts the tooltip on the temperature lane when it renders", () => {
    renderLanes(
      row({
        temperatureAvg: 55,
        cpuUsage: 40,
        powerAvg: 18,
      }),
      [40, 70],
      [0, 30],
    );

    const temperatureLane = screen.getByTestId("cooling-temperature-lane");
    expect(
      within(temperatureLane).getByText(
        "pages.insights.cooling.timeline.tooltip.average",
      ),
    ).toBeInTheDocument();
  });

  it("moves the tooltip to the load lane when no temperature was recorded", () => {
    // A machine whose temperature sensor is unavailable still archives CPU
    // load and package power. The temperature lane is not mounted, so
    // without a fallback owner nothing would carry the shared tooltip and
    // the two remaining lanes would have no readout at all.
    renderLanes(row({ cpuUsage: 40, powerAvg: 18 }), null, [0, 30]);

    const loadLane = screen.getByTestId("cooling-load-lane");
    expect(
      within(loadLane).getByText(
        "pages.insights.cooling.timeline.tooltip.cpuUsage",
      ),
    ).toBeInTheDocument();
    expect(
      within(loadLane).getByText(
        "pages.insights.cooling.timeline.tooltip.power",
      ),
    ).toBeInTheDocument();
    expect(within(loadLane).getByText("18.0 W")).toBeInTheDocument();
  });

  it("mounts exactly one shared tooltip whichever lane owns it", () => {
    // Two tooltips on one synchronized cursor would stack duplicate
    // readouts over each other.
    const { rerender } = renderLanes(
      row({ cpuUsage: 40, powerAvg: 18 }),
      null,
      [0, 30],
    );

    const readouts = () =>
      screen.getAllByText("pages.insights.cooling.timeline.tooltip.cpuUsage");
    expect(readouts()).toHaveLength(1);

    rerender(
      <TimelineLanes
        rows={[row({ temperatureAvg: 55, cpuUsage: 40, powerAvg: 18 })]}
        domain={[40, 70]}
        powerDomain={[0, 30]}
        fanRows={[]}
        fanSeries={[]}
        fanDomain={null}
        ambientRows={[]}
        ambientDomain={null}
        baseline={null}
        loadMode="usage"
        temperatureUnit="C"
      />,
    );
    expect(readouts()).toHaveLength(1);
  });

  it("reports the period as unrecorded rather than blank when it is empty", () => {
    renderLanes(row({}), null, null);

    const loadLane = screen.getByTestId("cooling-load-lane");
    expect(
      within(loadLane).getByText(
        "pages.insights.cooling.timeline.tooltip.noRecording",
      ),
    ).toBeInTheDocument();
  });
});

describe("TimelineLanes power lane", () => {
  it("does not mount the power lane without a power domain", () => {
    renderLanes(row({ temperatureAvg: 55, cpuUsage: 40 }), [40, 70], null);

    expect(screen.queryByTestId("cooling-power-lane")).not.toBeInTheDocument();
  });

  it("mounts the power lane once the period recorded watts", () => {
    renderLanes(
      row({ temperatureAvg: 55, cpuUsage: 40, powerAvg: 18 }),
      [40, 70],
      [0, 30],
    );

    expect(screen.getByTestId("cooling-power-lane")).toBeInTheDocument();
  });
});

describe("TimelineLanes ambient lane", () => {
  it("does not mount the ambient lane without an ambient domain", () => {
    // The machine every existing capture is taken on: no environmental
    // sensor, so no lane and no ambient rows in the tooltip either.
    renderLanes(row({ temperatureAvg: 55, cpuUsage: 40 }), [40, 70], [0, 30]);

    expect(
      screen.queryByTestId("cooling-ambient-lane"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByText("pages.insights.cooling.timeline.tooltip.ambient"),
    ).not.toBeInTheDocument();
  });

  it("mounts the ambient lane once the period recorded a room temperature", () => {
    renderLanes(
      row({ temperatureAvg: 55, cpuUsage: 40 }),
      [40, 70],
      [0, 30],
      NO_FAN,
      withAmbient({ ambient: 22, delta: 33 }),
    );

    expect(screen.getByTestId("cooling-ambient-lane")).toBeInTheDocument();
  });

  it("reports Core's paired delta rather than subtracting the two lanes", () => {
    // The bucket's CPU average is 55 and its ambient average 22, but the
    // two cover different sample sets - the tooltip must show the delta
    // Core paired per minute (33.4), not the 33.0 the rows would suggest.
    renderLanes(
      row({ temperatureAvg: 55, cpuUsage: 40 }),
      [40, 70],
      null,
      NO_FAN,
      withAmbient({ ambient: 22, delta: 33.4 }),
    );

    const temperatureLane = screen.getByTestId("cooling-temperature-lane");
    expect(within(temperatureLane).getByText("22.0°C")).toBeInTheDocument();
    expect(within(temperatureLane).getByText("33.4°C")).toBeInTheDocument();
  });

  it("leaves a period with ambient but no pairing without a delta row", () => {
    // An ambient minute whose CPU temperature was unreadable yields no ΔT
    // at all; the row is absent rather than showing the ambient value
    // again or a fabricated zero.
    renderLanes(
      row({ temperatureAvg: 55 }),
      [40, 70],
      null,
      NO_FAN,
      withAmbient({ ambient: 22, delta: null }),
    );

    const temperatureLane = screen.getByTestId("cooling-temperature-lane");
    expect(
      within(temperatureLane).getByText(
        "pages.insights.cooling.timeline.tooltip.ambient",
      ),
    ).toBeInTheDocument();
    expect(
      within(temperatureLane).queryByText(
        "pages.insights.cooling.timeline.tooltip.thermalDelta",
      ),
    ).not.toBeInTheDocument();
  });

  it("stops offering the load lane when it has nothing to show either", () => {
    // An ambient-only window degrades the temperature lane, but the load
    // lane below it is empty too, so the notice must not point at it.
    renderLanes(
      row({}),
      null,
      null,
      NO_FAN,
      withAmbient({ ambient: 22, delta: null }),
    );

    expect(
      screen.getByText(
        "pages.insights.cooling.timeline.temperatureUnavailableAlone",
      ),
    ).toBeInTheDocument();
  });

  it("still offers the load lane when the window recorded load", () => {
    renderLanes(
      row({ cpuUsage: 40 }),
      null,
      null,
      NO_FAN,
      withAmbient({ ambient: 22, delta: null }),
    );

    expect(
      screen.getByText(
        "pages.insights.cooling.timeline.temperatureUnavailable",
      ),
    ).toBeInTheDocument();
  });

  it("counts an ambient-only period as recorded rather than blank", () => {
    // The app was running and the room was measured; only the machine's
    // own sensors were silent.
    renderLanes(
      row({}),
      null,
      null,
      NO_FAN,
      withAmbient({ ambient: 22, delta: null }),
    );

    const loadLane = screen.getByTestId("cooling-load-lane");
    expect(within(loadLane).getByText("22.0°C")).toBeInTheDocument();
    expect(
      within(loadLane).queryByText(
        "pages.insights.cooling.timeline.tooltip.noRecording",
      ),
    ).not.toBeInTheDocument();
  });
});

describe("TimelineLanes fan lane", () => {
  it("does not mount the fan lane without a fan domain", () => {
    renderLanes(row({ temperatureAvg: 55, cpuUsage: 40 }), [40, 70], [0, 30]);

    expect(screen.queryByTestId("cooling-fan-lane")).not.toBeInTheDocument();
  });

  it("mounts the fan lane once the period recorded a fan", () => {
    renderLanes(
      row({ temperatureAvg: 55, cpuUsage: 40 }),
      [40, 70],
      [0, 30],
      withFans({ fan0: 900 }),
    );

    expect(screen.getByTestId("cooling-fan-lane")).toBeInTheDocument();
  });

  it("reports every fan by its own source in the shared tooltip", () => {
    // Folding the fans into one entry would answer a question nobody
    // asked: which fan spun up is the reading.
    renderLanes(
      row({ temperatureAvg: 55, cpuUsage: 40 }),
      [40, 70],
      null,
      withFans({ fan0: 912 }),
    );

    const temperatureLane = screen.getByTestId("cooling-temperature-lane");
    expect(within(temperatureLane).getByText("Fan 1")).toBeInTheDocument();
    expect(within(temperatureLane).getByText("912 rpm")).toBeInTheDocument();
  });

  it("reports an Inactive Fan Reading rather than treating it as unrecorded", () => {
    // 0 RPM means the fan is not reporting rotation - a measurement, and
    // the one a user checking a suspect fan is looking for.
    renderLanes(row({}), null, null, withFans({ fan0: 0 }));

    const loadLane = screen.getByTestId("cooling-load-lane");
    expect(within(loadLane).getByText("0 rpm")).toBeInTheDocument();
    expect(
      within(loadLane).queryByText(
        "pages.insights.cooling.timeline.tooltip.noRecording",
      ),
    ).not.toBeInTheDocument();
  });

  it("leaves a period the fan did not record out of the tooltip", () => {
    renderLanes(
      row({ temperatureAvg: 55 }),
      [40, 70],
      null,
      withFans({ fan0: null }),
    );

    const temperatureLane = screen.getByTestId("cooling-temperature-lane");
    expect(
      within(temperatureLane).queryByText("Fan 1"),
    ).not.toBeInTheDocument();
  });
});
