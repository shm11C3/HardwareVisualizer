import type {
  CoolingBandComparisonEntry,
  CoolingLoadBand,
  TemperatureUnit,
} from "@/rspc/bindings";
import { convertTemperatureDelta } from "./temperatureUnit";
import { toDisplayTemperature } from "./thermalTimeline";

/** One load band's baseline-vs-recent comparison, in display units. */
export type LoadBandDumbbellRow =
  | { band: CoolingLoadBand; comparable: false }
  | {
      band: CoolingLoadBand;
      comparable: true;
      baseline: number;
      recent: number;
      delta: number;
    };

/**
 * Convert Core's per-band comparison into display-ready rows. `comparable`
 * is Core's own fact (see `CoolingBandComparisonEntry.comparable`); a band
 * is also folded into the non-comparable row shape if either temperature is
 * unexpectedly missing despite that flag, since a dumbbell needs both ends
 * to draw a line.
 */
export const buildLoadBandDumbbellRows = (
  bands: readonly CoolingBandComparisonEntry[],
  temperatureUnit: TemperatureUnit,
): LoadBandDumbbellRow[] =>
  bands.map((entry) => {
    if (
      !entry.comparable ||
      entry.baseline.temperatureAvg == null ||
      entry.recent.temperatureAvg == null
    ) {
      return { band: entry.band, comparable: false };
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
      return { band: entry.band, comparable: false };
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
    if (
      adjusted == null ||
      !adjusted.comparable ||
      adjusted.baseline.deltaAvg == null ||
      adjusted.recent.deltaAvg == null
    ) {
      return { band: entry.band, comparable: false };
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
