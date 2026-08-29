import type { TemperatureUnit } from "@/rspc/bindings";

/**
 * Convert a temperature *difference* (not an absolute reading) between
 * display units. Unlike `toDisplayTemperature` in `thermalTimeline.ts`, a
 * delta carries no +32 offset - only the 9/5 scale factor applies, since
 * converting a span of degrees is not the same operation as converting a
 * point on the scale.
 */
export const convertTemperatureDelta = (
  deltaCelsius: number,
  unit: TemperatureUnit,
): number => (unit === "F" ? (deltaCelsius * 9) / 5 : deltaCelsius);

const MINUS_SIGN = "−";

/**
 * Render a temperature delta with an explicit sign, using a typographic
 * minus (U+2212) rather than a hyphen for negative values. `delta` is
 * already in the caller's display unit (see `convertTemperatureDelta`);
 * `unitSuffix` is appended as-is (e.g. `"°C"`).
 *
 * A delta that rounds to zero is shown with a leading "+" rather than a
 * negative sign, since `-0.0` reads as a rendering glitch, not a fact.
 */
export const formatSignedTemperatureDelta = (
  delta: number,
  unitSuffix: string,
): string => {
  const rounded = Number(delta.toFixed(1));
  const sign = rounded < 0 ? MINUS_SIGN : "+";
  return `${sign}${Math.abs(rounded).toFixed(1)}${unitSuffix}`;
};
