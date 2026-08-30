import type { CoolingDeltaObservation, TemperatureUnit } from "@/rspc/bindings";
import { convertTemperatureDelta } from "./temperatureUnit";

/**
 * The visual weight of an observation: which color the strip's status dot
 * takes. Not itself a verdict - Core already decided that (see
 * `CoolingDeltaObservation` in `core/src/persistence/cooling_baseline_delta.rs`) -
 * this only names which tone renders which state.
 */
export type ObservationTone = "muted" | "positive" | "mild" | "large";

/**
 * What `ObservationStrip` renders once a baseline is established, derived
 * from `CoolingDeltaObservation` plus the delta/streak numbers Core already
 * computed. `establishing` is handled upstream by `resolveBaselineLifecycle`
 * (an established baseline can never itself report `observation:
 * "establishing"` - see `derive_baseline_delta`), so this type only covers
 * the remaining four states.
 */
export type ObservationDisplay =
  | { kind: "notComparable"; tone: "muted" }
  | { kind: "withinRange"; tone: "positive"; delta: number | null }
  | {
      kind: "sustainedMildRise";
      tone: "mild";
      delta: number | null;
      sustainedDays: number;
    }
  | {
      kind: "sustainedLargeRise";
      tone: "large";
      delta: number | null;
      sustainedDays: number;
    };

/**
 * Map Core's `observation` verdict to the strip's display data. This is a
 * pure lookup - it holds no threshold or classification logic of its own,
 * since that judgment (what counts as "mild" vs "large", how many days make
 * a rise "sustained") is Core's, not the frontend's.
 *
 * `deltaCelsius`/`sustainedDays` are `CoolingBaselineDelta.delta` and
 * `.sustainedDays`; `delta` converts to `temperatureUnit` (as a span, not a
 * point - see `convertTemperatureDelta`).
 */
export const resolveObservationDisplay = (
  observation: Exclude<CoolingDeltaObservation, "establishing">,
  deltaCelsius: number | null,
  sustainedDays: number,
  temperatureUnit: TemperatureUnit,
): ObservationDisplay => {
  if (observation === "notComparable") {
    return { kind: "notComparable", tone: "muted" };
  }

  // A missing delta stays missing: coercing to 0 would label unavailable
  // hardware data as a measured zero-degree difference (DP-02).
  const delta =
    deltaCelsius == null
      ? null
      : convertTemperatureDelta(deltaCelsius, temperatureUnit);

  if (observation === "withinRange") {
    return { kind: "withinRange", tone: "positive", delta };
  }
  if (observation === "sustainedMildRise") {
    return { kind: "sustainedMildRise", tone: "mild", delta, sustainedDays };
  }
  return { kind: "sustainedLargeRise", tone: "large", delta, sustainedDays };
};

/**
 * Inclusive day count between two `YYYY-MM-DD` dates, used to render the
 * comparison strip's "last N days" phrase from the recent window's own
 * dates rather than duplicating Core's `COOLING_BASELINE_RECENT_WINDOW_DAYS`
 * constant on the frontend.
 */
export const daysInclusive = (startIso: string, endIso: string): number => {
  const start = Date.parse(`${startIso}T00:00:00Z`);
  const end = Date.parse(`${endIso}T00:00:00Z`);
  return Math.round((end - start) / (24 * 60 * 60 * 1000)) + 1;
};
