import type {
  CoolingBandComparability,
  CoolingBandComparisonEntry,
  CoolingLoadBand,
  TemperatureUnit,
} from "@/rspc/bindings";
import { convertTemperatureDelta } from "./temperatureUnit";
import { toDisplayTemperature } from "./thermalTimeline";

/**
 * How far short of Core's minimum each window is, in minutes. `0` on a
 * side means that side already has enough; the raw counts stay beside the
 * remainder so the panel can also say "12 / 30 min" for a window that
 * will not fill in on its own (the baseline window is fixed once pinned).
 */
export type LoadBandShortfall = {
  required: number;
  baseline: { sampleMinutes: number; remaining: number };
  recent: { sampleMinutes: number; remaining: number };
};

/**
 * Why a band is withheld. Core decides whether a band is comparable; this
 * only carries its reason plus the numbers the copy needs, so a thin
 * window can say how far short it is rather than only that it is.
 */
export type LoadBandWithheldReason =
  | { kind: "tooFewSampleMinutes"; shortfall: LoadBandShortfall }
  /** The band never paired a minute with ambient in either window. */
  | { kind: "noAmbientPairing" }
  /** A value Core reported comparable is missing anyway; cannot draw. */
  | { kind: "missingValue" };

/** One load band's baseline-vs-recent comparison, in display units. */
export type LoadBandDumbbellRow =
  | { band: CoolingLoadBand; comparable: false; reason: LoadBandWithheldReason }
  | {
      band: CoolingLoadBand;
      comparable: true;
      baseline: number;
      recent: number;
      delta: number;
    };

const shortfall = (
  required: number,
  baselineMinutes: number,
  recentMinutes: number,
): LoadBandShortfall => ({
  required,
  baseline: {
    sampleMinutes: baselineMinutes,
    remaining: Math.max(0, required - baselineMinutes),
  },
  recent: {
    sampleMinutes: recentMinutes,
    remaining: Math.max(0, required - recentMinutes),
  },
});

/**
 * Map Core's non-comparable verdict onto a reason. Returns `null` for
 * `comparable`, so callers can still fall through to the missing-value
 * guard below.
 */
const withheldReason = (
  comparability: CoolingBandComparability,
  minutes: {
    required: number;
    baseline: number;
    recent: number;
  },
): LoadBandWithheldReason | null => {
  switch (comparability) {
    case "comparable":
      return null;
    case "tooFewSampleMinutes":
      return {
        kind: "tooFewSampleMinutes",
        shortfall: shortfall(
          minutes.required,
          minutes.baseline,
          minutes.recent,
        ),
      };
  }
};

/**
 * Convert Core's per-band comparison into display-ready rows.
 * `comparability` is Core's own fact (see
 * `CoolingBandComparisonEntry.comparability`); a band is also folded into
 * the non-comparable row shape if either temperature is unexpectedly
 * missing despite that verdict, since a dumbbell needs both ends to draw a
 * line.
 */
export const buildLoadBandDumbbellRows = (
  bands: readonly CoolingBandComparisonEntry[],
  temperatureUnit: TemperatureUnit,
): LoadBandDumbbellRow[] =>
  bands.map((entry) => {
    const reason = withheldReason(entry.comparability, {
      required: entry.requiredSampleMinutes,
      baseline: entry.baseline.sampleMinutes,
      recent: entry.recent.sampleMinutes,
    });
    if (reason != null) {
      return { band: entry.band, comparable: false, reason };
    }
    if (
      entry.baseline.temperatureAvg == null ||
      entry.recent.temperatureAvg == null
    ) {
      return {
        band: entry.band,
        comparable: false,
        reason: { kind: "missingValue" },
      };
    }

    const baseline = toDisplayTemperature(
      entry.baseline.temperatureAvg,
      temperatureUnit,
    );
    const recent = toDisplayTemperature(
      entry.recent.temperatureAvg,
      temperatureUnit,
    );
    if (baseline == null || recent == null) {
      return {
        band: entry.band,
        comparable: false,
        reason: { kind: "missingValue" },
      };
    }

    const delta = convertTemperatureDelta(
      entry.recent.temperatureAvg - entry.baseline.temperatureAvg,
      temperatureUnit,
    );

    return { band: entry.band, comparable: true, baseline, recent, delta };
  });

/**
 * The same per-band comparison read over the Thermal Delta instead of the
 * absolute temperature (#2046), so a rise the room explains can be told
 * apart from one the cooling explains.
 *
 * Returns `null` - not an empty array - when no band carries an ambient
 * reading at all. That is the normal state on a machine with no
 * environmental sensor, and it has to stay distinguishable from "ambient
 * data exists but this window is too thin", because only the former means
 * the panel should render exactly as it did before ambient existed.
 *
 * Every endpoint is converted with `convertTemperatureDelta` rather than
 * `toDisplayTemperature`: a ΔT is already a difference between two
 * temperatures, so the +32 offset would be applied to a span that never
 * had a zero point on the Fahrenheit scale.
 */
export const buildAmbientAdjustedDumbbellRows = (
  bands: readonly CoolingBandComparisonEntry[],
  temperatureUnit: TemperatureUnit,
): LoadBandDumbbellRow[] | null => {
  if (bands.every((entry) => entry.ambientAdjusted == null)) {
    return null;
  }

  return bands.map((entry) => {
    const adjusted = entry.ambientAdjusted;
    if (adjusted == null) {
      return {
        band: entry.band,
        comparable: false,
        reason: { kind: "noAmbientPairing" },
      };
    }
    const reason = withheldReason(adjusted.comparability, {
      required: adjusted.requiredSampleMinutes,
      baseline: adjusted.baseline.sampleMinutes,
      recent: adjusted.recent.sampleMinutes,
    });
    if (reason != null) {
      return { band: entry.band, comparable: false, reason };
    }
    if (
      adjusted.baseline.deltaAvg == null ||
      adjusted.recent.deltaAvg == null
    ) {
      return {
        band: entry.band,
        comparable: false,
        reason: { kind: "missingValue" },
      };
    }

    const baseline = convertTemperatureDelta(
      adjusted.baseline.deltaAvg,
      temperatureUnit,
    );
    const recent = convertTemperatureDelta(
      adjusted.recent.deltaAvg,
      temperatureUnit,
    );

    return {
      band: entry.band,
      comparable: true,
      baseline,
      recent,
      delta: convertTemperatureDelta(
        adjusted.recent.deltaAvg - adjusted.baseline.deltaAvg,
        temperatureUnit,
      ),
    };
  });
};

/**
 * Map a display-unit temperature onto a 0-100 horizontal position within
 * `domain`, clamped to the track. A degenerate domain (identical min/max,
 * e.g. only one comparable band) centers the point rather than dividing by
 * zero.
 */
export const positionPercent = (
  value: number,
  domain: readonly [number, number],
): number => {
  const [min, max] = domain;
  if (max <= min) {
    return 50;
  }
  return Math.min(100, Math.max(0, ((value - min) / (max - min)) * 100));
};
