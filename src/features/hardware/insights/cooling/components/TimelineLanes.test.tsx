import { cleanup, render, screen, within } from "@testing-library/react";
import { cloneElement, isValidElement, type ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
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

const renderLanes = (
  hovered: ThermalTimelineRow,
  domain: [number, number] | null,
  powerDomain: [number, number] | null,
) => {
  mocks.hoveredRow = hovered;
  return render(
    <TimelineLanes
      rows={[hovered]}
      domain={domain}
      powerDomain={powerDomain}
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
