import type {
  CoolingAmbientAdjustedBaselineDelta,
  CoolingDeltaObservation,
  TemperatureUnit,
} from "@/rspc/bindings";
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
 * What the strip adds once the ambient-normalized reading of the same drift
 * is worth showing (#2046).
 *
 * Deliberately not a second `ObservationDisplay`: Core classifies the
 * absolute drift (`CoolingDeltaObservation`) but publishes no verdict and no
 * sustained-day streak for the ΔT reading, only a delta and whether the two
 * windows were comparable. Re-classifying that delta here - or borrowing the
 * absolute observation's streak, which was counted over a different series -
 * would be the frontend deciding what counts as a rise, which is exactly what
 * `#1666` keeps behind Core's boundary. So this reports the number and says
 * which baseline it is against, and nothing more.
 */
export type AmbientAdjustedDisplay =
  | { kind: "hidden" }
  | { kind: "establishing"; qualifyingDays: number; requiredDays: number }
  | {
      kind: "comparable";
      delta: number;
      /** The ΔT baseline's own window, routinely not the absolute one's. */
      windowStartDate: string;
      windowEndDate: string;
    };

/**
 * Decide what the ambient-adjusted line reports, from Core's
 * `CoolingBaselineDelta.ambientAdjusted` alone.
 *
 * `hidden` is the answer for every machine that cannot support the claim,
 * and it is the common one: a machine with no environmental sensor reports
 * an establishing ΔT baseline at zero qualifying days, so gating the
 * progress line on `qualifyingDays > 0` is what keeps such a machine
 * rendering exactly as it did before ambient existed. Zero qualifying days
 * is not evidence that a sensor is warming up - it is the absence of any
 * evidence that one exists (DP-02).
 *
 * An established ΔT baseline whose recent window is still too thin is
 * likewise hidden rather than announced: "insufficient coverage" is a state
 * the strip already has no line for, and adding one would change the reading
 * for a machine whose ambient data simply has not accumulated yet.
 */
export const resolveAmbientAdjustedDisplay = (
  ambientAdjusted: CoolingAmbientAdjustedBaselineDelta | null | undefined,
  temperatureUnit: TemperatureUnit,
): AmbientAdjustedDisplay => {
  if (ambientAdjusted == null) {
    return { kind: "hidden" };
  }

  const { baseline, comparable, delta } = ambientAdjusted;

  if (baseline.status === "establishing") {
    return baseline.qualifyingDays > 0
      ? {
          kind: "establishing",
          qualifyingDays: baseline.qualifyingDays,
          requiredDays: baseline.requiredDays,
        }
      : { kind: "hidden" };
  }

  // Core's contract is that `delta` is non-null whenever `comparable`. A
  // response that contradicts it is not turned into a measured 0.0°.
  if (!comparable || delta == null) {
    return { kind: "hidden" };
  }

  return {
    kind: "comparable",
    delta: convertTemperatureDelta(delta, temperatureUnit),
    windowStartDate: baseline.windowStartDate,
    windowEndDate: baseline.windowEndDate,
  };
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
