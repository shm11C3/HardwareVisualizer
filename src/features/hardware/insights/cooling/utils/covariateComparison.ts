import type {
  CoolingCovariateComparison,
  CoolingFactorComparison,
  CoolingFactorJudgement,
  CoolingLeastSquaresFit,
  TemperatureUnit,
} from "@/rspc/bindings";
import { fanColor } from "./fanTimeline";
import {
  convertTemperatureDelta,
  formatSignedTemperatureDelta,
  MINUS_SIGN,
} from "./temperatureUnit";

export type EstablishedCovariateComparison = Extract<
  CoolingCovariateComparison,
  { status: "established" }
>;

/**
 * The tag a factor row carries. The first four are Core's judgement
 * translated one-to-one; `removedByDelta` is the ambient row's, because its
 * movement is already subtracted out of every Thermal Delta in the table;
 * `atMatchedPower` is the Thermal Delta row's, which Core does not judge at
 * all - it reports the change at the baseline's median power and stops.
 */
export type CovariateTag =
  | "moved"
  | "withinRange"
  | "notComparable"
  | "notArchived"
  | "removedByDelta"
  | "atMatchedPower";

export type CovariateFactorKind =
  | "thermalDelta"
  | "packagePower"
  | "fan"
  | "loadBandShare"
  | "ambient";

export type CovariateRow = {
  key: string;
  kind: CovariateFactorKind;
  /** The archived fan identifier; only fan rows carry one. */
  fanSource?: string;
  /** The color of the lane this factor is drawn in on the timeline. */
  color: string;
  /** Formatted display values; null where the window never archived it. */
  baseline: string | null;
  recent: string | null;
  change: string | null;
  tag: CovariateTag;
  /**
   * Core's judgement, for the lead sentence - null for the rows the lead
   * never lists (the Thermal Delta, which is the subject of the sentence,
   * and ambient, whose movement is removed rather than compared).
   */
  judgement: CoolingFactorJudgement | null;
};

/**
 * Lane tokens, as `TimelineLanes` assigns them: `--chart-1` temperature,
 * `--chart-3` power, `--chart-4` CPU load; fans cycle with `fanColor`.
 * The `--chart-N` tokens are bare HSL triplets and need the `hsl()` wrap;
 * `--muted-foreground` is already a complete color and must not get one.
 *
 * Ambient shares the temperature token on the timeline, but two identical
 * dots in one table would read as two rows of the same series. The ambient
 * row is not a compared series at all - its tag says its movement was
 * removed - so it takes the muted foreground instead.
 */
export const covariateColors = {
  thermalDelta: "hsl(var(--chart-1))",
  packagePower: "hsl(var(--chart-3))",
  loadBandShare: "hsl(var(--chart-4))",
  ambient: "var(--muted-foreground)",
} as const;

/** The baseline fit is drawn in the muted foreground, the recent in `--chart-1`. */
export const fitLineColors = {
  baseline: "var(--muted-foreground)",
  recent: "hsl(var(--chart-1))",
} as const;

const formatQuantity = (
  value: number | null,
  fractionDigits: number,
  unitSuffix: string,
): string | null =>
  value == null ? null : `${value.toFixed(fractionDigits)}${unitSuffix}`;

/**
 * A signed change with a typographic minus, like
 * `formatSignedTemperatureDelta` but for the non-temperature factors.
 */
export const formatSignedQuantity = (
  value: number,
  fractionDigits: number,
  unitSuffix: string,
): string => {
  const rounded = Number(value.toFixed(fractionDigits));
  const sign = rounded < 0 ? MINUS_SIGN : "+";
  return `${sign}${Math.abs(rounded).toFixed(fractionDigits)}${unitSuffix}`;
};

const TAG_FOR_JUDGEMENT: Record<CoolingFactorJudgement, CovariateTag> = {
  moved: "moved",
  withinRange: "withinRange",
  notComparable: "notComparable",
  absent: "notArchived",
};

type QuantityFormat = {
  fractionDigits: number;
  unitSuffix: string;
  changeSuffix: string;
};

const POWER_FORMAT: QuantityFormat = {
  fractionDigits: 1,
  unitSuffix: " W",
  changeSuffix: " W",
};
const FAN_FORMAT: QuantityFormat = {
  fractionDigits: 0,
  unitSuffix: " rpm",
  changeSuffix: " rpm",
};
/** A share is a percentage; its change is a difference in points. */
const SHARE_FORMAT: QuantityFormat = {
  fractionDigits: 1,
  unitSuffix: " %",
  changeSuffix: " pt",
};

const quantityRow = (
  factor: CoolingFactorComparison,
  format: QuantityFormat,
): Pick<CovariateRow, "baseline" | "recent" | "change" | "tag"> => ({
  baseline: formatQuantity(
    factor.baseline,
    format.fractionDigits,
    format.unitSuffix,
  ),
  recent: formatQuantity(
    factor.recent,
    format.fractionDigits,
    format.unitSuffix,
  ),
  change:
    factor.change == null
      ? null
      : formatSignedQuantity(
          factor.change,
          format.fractionDigits,
          format.changeSuffix,
        ),
  tag: TAG_FOR_JUDGEMENT[factor.judgement],
});

export const temperatureUnitSuffix = (unit: TemperatureUnit): string =>
  unit === "C" ? "°C" : "°F";

/** The ΔT the line reads at `power`, in the unit Core fitted it in (K). */
const fitAt = (fit: CoolingLeastSquaresFit, power: number): number =>
  fit.slope * power + fit.intercept;

const formatDisplayDelta = (
  deltaKelvin: number,
  unit: TemperatureUnit,
): string =>
  formatSignedTemperatureDelta(
    convertTemperatureDelta(deltaKelvin, unit),
    temperatureUnitSuffix(unit),
  );

/**
 * The Thermal Delta row: not a factor Core judges, so it exists only when
 * every value it shows is one Core produced - both fits, the baseline's
 * median power to read them at, and the change at that power. Anything
 * less and the row is omitted rather than filled with a guess.
 */
const thermalDeltaRow = (
  comparison: EstablishedCovariateComparison,
  unit: TemperatureUnit,
): CovariateRow | null => {
  const { baselineFit, recentFit, deltaAtBaselineMedianPower } = comparison;
  const power = comparison.packagePower.baseline;
  if (
    baselineFit == null ||
    recentFit == null ||
    power == null ||
    deltaAtBaselineMedianPower == null
  ) {
    return null;
  }
  const suffix = temperatureUnitSuffix(unit);
  return {
    key: "thermalDelta",
    kind: "thermalDelta",
    color: covariateColors.thermalDelta,
    baseline: formatQuantity(
      convertTemperatureDelta(fitAt(baselineFit, power), unit),
      1,
      suffix,
    ),
    recent: formatQuantity(
      convertTemperatureDelta(fitAt(recentFit, power), unit),
      1,
      suffix,
    ),
    change: formatDisplayDelta(deltaAtBaselineMedianPower, unit),
    tag: "atMatchedPower",
    judgement: null,
  };
};

/**
 * Ambient is archived in Celsius, and its change is a delta: the recent
 * median against the baseline's, so it converts without the +32 offset.
 */
const ambientRow = (
  factor: CoolingFactorComparison,
  unit: TemperatureUnit,
): CovariateRow => {
  const suffix = temperatureUnitSuffix(unit);
  const toDisplay = (celsius: number | null) =>
    celsius == null ? null : unit === "F" ? (celsius * 9) / 5 + 32 : celsius;
  return {
    key: "ambient",
    kind: "ambient",
    color: covariateColors.ambient,
    baseline: formatQuantity(toDisplay(factor.baseline), 1, suffix),
    recent: formatQuantity(toDisplay(factor.recent), 1, suffix),
    change:
      factor.change == null ? null : formatDisplayDelta(factor.change, unit),
    // Whatever the room did is already subtracted from every ΔT in the
    // table; the row is here so the reader can see it was, not compared.
    tag: factor.judgement === "absent" ? "notArchived" : "removedByDelta",
    judgement: null,
  };
};

/**
 * One row per archived co-variate, in reading order: the Thermal Delta
 * itself, then the operating-point factors (power, each fan, load share),
 * then ambient. A factor a window never archived keeps null values, which
 * the table renders as a dash - never as 0 W or 0 rpm (DP-02).
 */
export const buildCovariateRows = (
  comparison: EstablishedCovariateComparison,
  unit: TemperatureUnit,
): CovariateRow[] => {
  const rows: CovariateRow[] = [];
  const thermalDelta = thermalDeltaRow(comparison, unit);
  if (thermalDelta != null) {
    rows.push(thermalDelta);
  }
  rows.push({
    key: "packagePower",
    kind: "packagePower",
    color: covariateColors.packagePower,
    ...quantityRow(comparison.packagePower, POWER_FORMAT),
    judgement: comparison.packagePower.judgement,
  });
  comparison.fans.forEach((fan, index) => {
    rows.push({
      key: `fan:${fan.fanSource}`,
      kind: "fan",
      fanSource: fan.fanSource,
      color: fanColor(index),
      ...quantityRow(fan.speed, FAN_FORMAT),
      judgement: fan.speed.judgement,
    });
  });
  rows.push({
    key: "loadBandShare",
    kind: "loadBandShare",
    color: covariateColors.loadBandShare,
    ...quantityRow(comparison.loadBandShare, SHARE_FORMAT),
    judgement: comparison.loadBandShare.judgement,
  });
  rows.push(ambientRow(comparison.ambientTemperature, unit));
  return rows;
};

/**
 * What the lead sentence says, as data: the ΔT change at matched power
 * (null when Core produced none, and the clause is then simply absent),
 * then the rows Core judged `moved`, then those judged `withinRange`.
 */
export type CovariateLead = {
  deltaAtMatchedPower: string | null;
  moved: CovariateRow[];
  withinRange: CovariateRow[];
};

export const buildCovariateLead = (
  comparison: EstablishedCovariateComparison,
  rows: readonly CovariateRow[],
  unit: TemperatureUnit,
): CovariateLead => ({
  deltaAtMatchedPower:
    comparison.deltaAtBaselineMedianPower == null
      ? null
      : formatDisplayDelta(comparison.deltaAtBaselineMedianPower, unit),
  moved: rows.filter((row) => row.judgement === "moved"),
  withinRange: rows.filter((row) => row.judgement === "withinRange"),
});

/**
 * How far past the two windows' median powers the fitted lines are drawn,
 * as a share of each median.
 *
 * The DTO carries the fits but no paired minutes, so the observed power
 * range is inferred rather than known: the band's medians are the one
 * anchor Core gives, and half a median either side of them covers the
 * spread an idle band shows in practice without pretending to know its
 * extremes. The lines are fits, and the caption says so.
 */
export const FIT_LINE_SPAN_RATIO = 0.5;

export type FitLineRow = {
  /** Package power, W. */
  x: number;
  /** The fitted ΔT in the display unit, null where a window has no fit. */
  baseline: number | null;
  recent: number | null;
};

export type FitLineChart = {
  /** The power axis, W. */
  domain: [number, number];
  /** The two endpoints of each fitted line. */
  rows: FitLineRow[];
  /** The baseline window's median power, W - where the lead reads ΔT. */
  anchorPower: number;
  /** Formatted `slope unit/W` legend labels, null where a window has no fit. */
  baselineSlope: string | null;
  recentSlope: string | null;
};

const slopeUnit = (unit: TemperatureUnit): string =>
  unit === "C" ? "K/W" : "°F/W";

export const formatFitSlope = (
  fit: CoolingLeastSquaresFit,
  unit: TemperatureUnit,
): string =>
  `${convertTemperatureDelta(fit.slope, unit).toFixed(2)} ${slopeUnit(unit)}`;

/**
 * The two fitted lines across the inferred power range, or null when
 * there is nothing to anchor them on: no baseline median power, or no fit
 * in either window. A missing fit leaves its line out rather than drawing
 * a flat one.
 */
export const buildFitLineChart = (
  comparison: EstablishedCovariateComparison,
  unit: TemperatureUnit,
): FitLineChart | null => {
  const { baselineFit, recentFit, packagePower } = comparison;
  const anchorPower = packagePower.baseline;
  if (anchorPower == null || (baselineFit == null && recentFit == null)) {
    return null;
  }
  const anchors = [anchorPower, packagePower.recent].filter(
    (value): value is number => value != null,
  );
  const xMin = Math.max(
    0,
    Math.floor(Math.min(...anchors) * (1 - FIT_LINE_SPAN_RATIO)),
  );
  const xMax = Math.ceil(Math.max(...anchors) * (1 + FIT_LINE_SPAN_RATIO));
  const displayAt = (fit: CoolingLeastSquaresFit | null, x: number) =>
    fit == null
      ? null
      : Number(convertTemperatureDelta(fitAt(fit, x), unit).toFixed(2));

  return {
    domain: [xMin, xMax],
    rows: [xMin, xMax].map((x) => ({
      x,
      baseline: displayAt(baselineFit, x),
      recent: displayAt(recentFit, x),
    })),
    anchorPower,
    baselineSlope:
      baselineFit == null ? null : formatFitSlope(baselineFit, unit),
    recentSlope: recentFit == null ? null : formatFitSlope(recentFit, unit),
  };
};
