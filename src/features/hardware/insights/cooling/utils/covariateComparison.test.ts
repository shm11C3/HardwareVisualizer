import { describe, expect, it } from "vitest";
import {
  buildCovariateLead,
  buildCovariateRows,
  buildFitLineChart,
  type EstablishedCovariateComparison,
  formatFitSlope,
} from "./covariateComparison";

const LABELS = { pointsSuffix: " pt" };

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

const comparison = (
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
  loadBandShare: factor(0.62, 0.68, "withinRange"),
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
  // recentFit(18.4) - baselineFit(18.4)
  deltaAtBaselineMedianPower: 4.064,
  comparable: true,
  comparability: "comparable",
  ...overrides,
});

describe("buildCovariateRows", () => {
  it("lists the Thermal Delta, power, each fan, load share and ambient in that order", () => {
    const keys = buildCovariateRows(comparison(), "C", LABELS).map(
      (row) => row.key,
    );

    expect(keys).toEqual([
      "thermalDelta",
      "packagePower",
      "fan:CPU fan",
      "fan:case fan 2",
      "loadBandShare",
      "ambient",
    ]);
  });

  it("reads the Thermal Delta row off both fits at the baseline's median power", () => {
    const [thermalDelta] = buildCovariateRows(comparison(), "C", LABELS);

    // 1.31 * 18.4 + 4 = 28.104; 1.52 * 18.4 + 4.2 = 32.168.
    expect(thermalDelta).toMatchObject({
      baseline: "28.1°C",
      recent: "32.2°C",
      change: "+4.1°C",
      tag: "atMatchedPower",
      judgement: null,
    });
  });

  it("omits the Thermal Delta row rather than guessing when a fit is missing", () => {
    const rows = buildCovariateRows(
      comparison({ recentFit: null, deltaAtBaselineMedianPower: null }),
      "C",
      LABELS,
    );

    expect(rows.some((row) => row.kind === "thermalDelta")).toBe(false);
  });

  it("keeps an absent factor's values null instead of rendering a zero", () => {
    const rows = buildCovariateRows(comparison(), "C", LABELS);
    const absentFan = rows.find((row) => row.key === "fan:case fan 2");

    expect(absentFan).toMatchObject({
      baseline: null,
      recent: null,
      change: null,
      tag: "notArchived",
      judgement: "absent",
    });
  });

  it("formats each factor in its own unit, with a share's change in points", () => {
    const rows = buildCovariateRows(comparison(), "C", LABELS);
    const byKey = Object.fromEntries(rows.map((row) => [row.key, row]));

    expect(byKey["packagePower"]).toMatchObject({
      baseline: "18.4 W",
      recent: "19.1 W",
      change: "+0.7 W",
      tag: "withinRange",
    });
    expect(byKey["fan:CPU fan"]).toMatchObject({
      baseline: "1180 rpm",
      recent: "970 rpm",
      change: "−210 rpm",
      tag: "moved",
    });
    expect(byKey["loadBandShare"]).toMatchObject({
      baseline: "62.0 %",
      recent: "68.0 %",
      change: "+6.0 pt",
    });
  });

  it("tags ambient as removed by the Thermal Delta and keeps it out of the lead", () => {
    const rows = buildCovariateRows(comparison(), "C", LABELS);
    const ambient = rows.find((row) => row.kind === "ambient");

    // Core judged the room "moved", and the table still does not list it
    // as a co-variate that moved: its movement is what ΔT subtracts.
    expect(ambient).toMatchObject({
      baseline: "23.4°C",
      recent: "27.1°C",
      change: "+3.7°C",
      tag: "removedByDelta",
      judgement: null,
    });
  });

  it("converts the ambient reading as a temperature and its change as a delta", () => {
    const rows = buildCovariateRows(comparison(), "F", LABELS);
    const ambient = rows.find((row) => row.kind === "ambient");

    // 23.4 degC -> 74.1 degF (with offset); +3.7 K -> +6.7 degF (no offset).
    expect(ambient).toMatchObject({
      baseline: "74.1°F",
      recent: "80.8°F",
      change: "+6.7°F",
    });
  });
});

describe("buildCovariateLead", () => {
  it("partitions the judged rows into moved and within range", () => {
    const rows = buildCovariateRows(comparison(), "C", LABELS);
    const lead = buildCovariateLead(comparison(), rows, "C");

    expect(lead.deltaAtMatchedPower).toBe("+4.1°C");
    expect(lead.moved.map((row) => row.key)).toEqual(["fan:CPU fan"]);
    expect(lead.withinRange.map((row) => row.key)).toEqual([
      "packagePower",
      "loadBandShare",
    ]);
  });

  it("leaves the matched-power clause out when Core produced no delta", () => {
    const source = comparison({ deltaAtBaselineMedianPower: null });
    const rows = buildCovariateRows(source, "C", LABELS);

    expect(
      buildCovariateLead(source, rows, "C").deltaAtMatchedPower,
    ).toBeNull();
  });
});

describe("buildFitLineChart", () => {
  it("spans half a median either side of the two windows' median powers", () => {
    const chart = buildFitLineChart(comparison(), "C");

    // min(18.4, 19.1) * 0.5 = 9.2 -> 9; max * 1.5 = 28.65 -> 29.
    expect(chart?.domain).toEqual([9, 29]);
    expect(chart?.anchorPower).toBe(18.4);
    expect(chart?.rows).toEqual([
      { x: 9, baseline: 15.79, recent: 17.88 },
      { x: 29, baseline: 41.99, recent: 48.28 },
    ]);
  });

  it("labels each line with its slope in kelvin per watt", () => {
    const chart = buildFitLineChart(comparison(), "C");

    expect(chart?.baselineSlope).toBe("1.31 K/W");
    expect(chart?.recentSlope).toBe("1.52 K/W");
  });

  it("scales the lines and slopes by 9/5 for Fahrenheit without an offset", () => {
    const chart = buildFitLineChart(comparison(), "F");

    expect(chart?.baselineSlope).toBe("2.36 °F/W");
    expect(chart?.rows[0]?.baseline).toBeCloseTo(15.79 * 1.8, 1);
  });

  it("draws only the window that has a fit", () => {
    const chart = buildFitLineChart(comparison({ recentFit: null }), "C");

    expect(chart?.recentSlope).toBeNull();
    expect(chart?.rows.every((row) => row.recent == null)).toBe(true);
  });

  it("draws nothing without a median power to anchor on", () => {
    expect(
      buildFitLineChart(
        comparison({ packagePower: factor(null, null, "absent") }),
        "C",
      ),
    ).toBeNull();
  });

  it("draws nothing when neither window has a fit", () => {
    expect(
      buildFitLineChart(
        comparison({ baselineFit: null, recentFit: null }),
        "C",
      ),
    ).toBeNull();
  });
});

describe("formatFitSlope", () => {
  it("keeps two decimals so a 0.2 K/W difference between windows stays visible", () => {
    expect(
      formatFitSlope(
        { slope: 1.3149, intercept: 0, pearsonR: 0, pairedMinutes: 2 },
        "C",
      ),
    ).toBe("1.31 K/W");
  });
});

describe("buildCovariateRows - tags and colors that must agree with the rest of the tab", () => {
  it("colors a fan by its position in the lane's sorted order, whatever order Core listed it in", () => {
    const rows = buildCovariateRows(
      comparison({
        // Core's BTreeSet order is byte order, so a capital sorts first;
        // the lane sorts with localeCompare, where "case fan 2" precedes
        // "CPU fan". The two orders disagree exactly here.
        fans: [
          {
            fanSource: "CPU fan",
            speed: factor(1180, 970, "moved"),
            baselineFit: null,
            recentFit: null,
          },
          {
            fanSource: "case fan 2",
            speed: factor(800, 810, "withinRange"),
            baselineFit: null,
            recentFit: null,
          },
        ],
      }),
      "C",
      LABELS,
    );
    const byKey = Object.fromEntries(rows.map((row) => [row.key, row]));

    expect(byKey["fan:case fan 2"].color).toBe("hsl(var(--chart-2))");
    expect(byKey["fan:CPU fan"].color).toBe("hsl(var(--chart-5))");
  });

  it("keeps Core's not-comparable verdict on the ambient row instead of calling it removed", () => {
    const rows = buildCovariateRows(
      comparison({ ambientTemperature: factor(23.4, 27.1, "notComparable") }),
      "C",
      LABELS,
    );

    expect(rows.find((row) => row.kind === "ambient")?.tag).toBe(
      "notComparable",
    );
  });

  it("prints the share change with the suffix the caller translated", () => {
    const rows = buildCovariateRows(comparison(), "C", {
      pointsSuffix: " ポイント",
    });

    expect(rows.find((row) => row.kind === "loadBandShare")?.change).toBe(
      "+6.0 ポイント",
    );
  });
});
