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
