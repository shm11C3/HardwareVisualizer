import { cleanup, render, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type {
  CoolingCovariateComparison,
  TemperatureUnit,
} from "@/rspc/bindings";
import type { EstablishedCovariateComparison } from "../utils/covariateComparison";
import { CovariateComparisonPanel } from "./CovariateComparisonPanel";

const mocks = vi.hoisted(() => ({
  getCoolingCovariateComparison: vi.fn(),
  dialogError: vi.fn(),
  settings: { temperatureUnit: "C" as TemperatureUnit },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // Keys with their interpolations, so a test can see what a fragment
    // was rendered with without reproducing the English copy.
    t: (key: string, options?: Record<string, unknown>) =>
      options == null ? key : `${key}:${JSON.stringify(options)}`,
    i18n: { language: "en" },
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({ settings: mocks.settings }),
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: mocks.dialogError }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getCoolingCovariateComparison: mocks.getCoolingCovariateComparison,
  },
}));

vi.mock("recharts", () => ({
  ResponsiveContainer: ({ children }: { children: ReactNode }) => (
    <>{children}</>
  ),
  LineChart: ({ children }: { children: ReactNode }) => <>{children}</>,
  Line: () => null,
  CartesianGrid: () => null,
  ReferenceLine: () => null,
  XAxis: () => null,
  YAxis: () => null,
}));

const factor = (
  baseline: number | null,
  recent: number | null,
  judgement: EstablishedCovariateComparison["packagePower"]["judgement"],
) => ({
  baseline,
  recent,
  change: baseline == null || recent == null ? null : recent - baseline,
  judgement,
});

const established = (
  overrides: Partial<EstablishedCovariateComparison> = {},
): EstablishedCovariateComparison => ({
  status: "established",
  band: "idle",
  baselineSource: "meter",
  baselineWindowStartDate: "2025-12-01",
  baselineWindowEndDate: "2025-12-14",
  recentSource: "meter",
  recentWindowStartDate: "2026-01-09",
  recentWindowEndDate: "2026-01-15",
  baselinePairedMinutes: 1_240,
  recentPairedMinutes: 1_105,
  packagePower: factor(18.4, 19.1, "withinRange"),
  ambientTemperature: factor(23.4, 27.1, "moved"),
  loadBandShare: factor(62, 68, "withinRange"),
  fans: [
    {
      fanSource: "CPU fan",
      speed: factor(1_180, 970, "moved"),
      baselineFit: null,
      recentFit: null,
    },
    {
      fanSource: "case fan 2",
      speed: factor(null, null, "absent"),
      baselineFit: null,
      recentFit: null,
    },
  ],
  baselineFit: {
    slope: 1.31,
    intercept: 4,
    pearsonR: 0.9,
    pairedMinutes: 1_240,
  },
  recentFit: {
    slope: 1.52,
    intercept: 4.2,
    pearsonR: 0.92,
    pairedMinutes: 1_105,
  },
  deltaAtBaselineMedianPower: 4.064,
  comparable: true,
  comparability: "comparable",
  ...overrides,
});

const resolveWith = (comparison: CoolingCovariateComparison) => {
  mocks.getCoolingCovariateComparison.mockResolvedValue({
    status: "ok",
    data: comparison,
  });
};

const KEY = "pages.insights.cooling.covariateComparison";

beforeEach(() => {
  vi.clearAllMocks();
  mocks.settings.temperatureUnit = "C";
});

afterEach(cleanup);

describe("CovariateComparisonPanel", () => {
  it("renders the establishing line while the Thermal Delta Baseline is still forming", async () => {
    resolveWith({ status: "establishing", qualifyingDays: 2, requiredDays: 3 });

    render(<CovariateComparisonPanel ambientCapability="present" />);

    await waitFor(() => {
      expect(
        screen.getByText(
          'pages.insights.cooling.dataState.establishing:{"qualifyingDays":2,"requiredDays":3}',
        ),
      ).toBeInTheDocument();
    });
    expect(screen.queryByTestId("cooling-covariate-lead")).toBeNull();
  });

  it("renders nothing for a machine whose baseline has never qualified a day", async () => {
    // The zero-day answer is how a machine with no environmental sensor
    // reports itself on the long-range routes, where the capability is
    // unknown; it is the same gate the strip's ambient line uses.
    resolveWith({ status: "establishing", qualifyingDays: 0, requiredDays: 3 });

    render(<CovariateComparisonPanel ambientCapability="unknown" />);

    await waitFor(() => {
      expect(mocks.getCoolingCovariateComparison).toHaveBeenCalledOnce();
    });
    await waitFor(() => {
      expect(screen.queryByTestId("cooling-covariate-panel")).toBeNull();
    });
  });

  it("names the reason and skips the lead sentence when the windows are not comparable", async () => {
    resolveWith(
      established({
        comparable: false,
        comparability: "differentAmbientSource",
        deltaAtBaselineMedianPower: null,
      }),
    );

    render(<CovariateComparisonPanel ambientCapability="present" />);

    await waitFor(() => {
      expect(
        screen.getByTestId("cooling-covariate-not-comparable"),
      ).toHaveTextContent(`${KEY}.notComparable.differentAmbientSource`);
    });
    expect(screen.queryByTestId("cooling-covariate-lead")).toBeNull();
    expect(screen.queryByTestId("cooling-covariate-chart")).toBeNull();
  });

  it("renders a dash and the not-archived tag for a factor no window archived", async () => {
    resolveWith(established());

    render(<CovariateComparisonPanel ambientCapability="present" />);

    const row = await screen.findByText(
      `${KEY}.factors.fan:{"fanSource":"case fan 2"}`,
    );
    const cells = row.closest("tr")?.querySelectorAll("td") ?? [];
    expect(cells[1]).toHaveTextContent("—");
    expect(cells[2]).toHaveTextContent("—");
    expect(cells[3]).toHaveTextContent("—");
    expect(cells[4]).toHaveTextContent(`${KEY}.tags.notArchived`);
    expect(row.closest("tr")).not.toHaveTextContent("0 rpm");
  });

  it("tags a moved factor and lists it, with its change, in the lead sentence", async () => {
    resolveWith(established());

    render(<CovariateComparisonPanel ambientCapability="present" />);

    const lead = await screen.findByTestId("cooling-covariate-lead");
    expect(lead).toHaveTextContent(
      `${KEY}.lead.deltaAtMatchedPower:{"delta":"+4.1°C"}`,
    );
    // The moved clause carries the fan by name with its change; the
    // mocked `t` nests the fragments as escaped JSON, so match the parts.
    expect(lead).toHaveTextContent(`${KEY}.lead.moved:`);
    expect(lead).toHaveTextContent(`${KEY}.lead.factorWithChange:`);
    expect(lead).toHaveTextContent("CPU fan");
    expect(lead).toHaveTextContent("−210 rpm");
    // Within range names the factors without a change; ambient is in
    // neither list.
    expect(lead).toHaveTextContent(`${KEY}.lead.withinRange:`);
    expect(lead).not.toHaveTextContent(`${KEY}.factors.ambient`);

    const fanRow = screen
      .getByText(`${KEY}.factors.fan:{"fanSource":"CPU fan"}`)
      .closest("tr");
    expect(fanRow).toHaveTextContent(`${KEY}.tags.moved`);
  });

  it("converts the Thermal Delta and the slope labels for Fahrenheit", async () => {
    mocks.settings.temperatureUnit = "F";
    resolveWith(established());

    render(<CovariateComparisonPanel ambientCapability="present" />);

    const lead = await screen.findByTestId("cooling-covariate-lead");
    // +4.064 K * 9/5 = +7.3 degF, no offset.
    expect(lead).toHaveTextContent(
      `${KEY}.lead.deltaAtMatchedPower:{"delta":"+7.3°F"}`,
    );
    expect(
      screen.getByText(`${KEY}.chart.legend.baseline:{"slope":"2.36 °F/W"}`),
    ).toBeInTheDocument();
    expect(
      screen.getByText(`${KEY}.chart.legend.recent:{"slope":"2.74 °F/W"}`),
    ).toBeInTheDocument();
  });

  it("renders nothing, and does not fetch, when the window proves there is no ambient source", () => {
    resolveWith(established());

    const { container } = render(
      <CovariateComparisonPanel ambientCapability="absent" />,
    );

    expect(container).toBeEmptyDOMElement();
    expect(mocks.getCoolingCovariateComparison).not.toHaveBeenCalled();
  });

  it("reports a failed fetch instead of keeping the skeleton", async () => {
    mocks.getCoolingCovariateComparison.mockResolvedValue({
      status: "error",
      error: "boom",
    });

    render(<CovariateComparisonPanel ambientCapability="present" />);

    await waitFor(() => {
      expect(screen.getByText(`${KEY}.loadFailed`)).toBeInTheDocument();
    });
    expect(screen.queryByTestId("cooling-covariate-panel-loading")).toBeNull();
  });
});
