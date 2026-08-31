import type { AmbientArchiveSeries, TemperatureUnit } from "@/rspc/bindings";
import { convertTemperatureDelta } from "./temperatureUnit";
import {
  type ArchiveTimelineSeries,
  archiveWindowRecordedAnything,
  computeSignedTemperatureDomain,
  type ThermalTimelineRow,
  toDisplayTemperature,
} from "./thermalTimeline";

/**
 * One column of the ambient lane, aligned to the timeline's shared axis.
 *
 * Carried on its own row array rather than on [`ThermalTimelineRow`] for
 * the reason the fan lane is: ambient arrives from its own command over its
 * own archive table, and the long-range routes have no ambient source at
 * all, so folding it into the row every other lane is built from would make
 * an absent capability look like a missing column.
 */
export type AmbientLaneRow = {
  /** The matching `ThermalTimelineRow.key`, so both lanes break together. */
  key: string;
  label: string;
  /** Ambient temperature for the period, in the display unit. */
  ambient: number | null;
  /**
   * Thermal Delta for the period - how far the CPU package sat above
   * ambient - in display degrees.
   *
   * Read straight off Core's `deltaAvg` and never recomputed here. Core
   * pairs each archived minute before averaging; subtracting this lane's
   * bucket average from the temperature lane's would aggregate two
   * different sample sets and produce a number matching no minute that was
   * ever observed (see `docs/architecture/backend.md`).
   */
  delta: number | null;
};

/**
 * Project Core's ambient buckets onto the timeline's own rows.
 *
 * Driven by `rows` rather than by the ambient buckets' own timestamps, so
 * the ambient lane always has the same length, labels and gaps as the lanes
 * above it - which is what keeps the synchronized cursor honest. A period
 * with no ambient row stays null and draws as a break.
 *
 * `series` is null on the long-range routes, which read the daily rollup:
 * it carries the per-band Thermal Delta but no ambient temperature series,
 * so there is nothing to draw and nothing is claimed either way.
 */
export const buildAmbientLaneRows = (
  rows: readonly ThermalTimelineRow[],
  series: AmbientArchiveSeries | null,
  temperatureUnit: TemperatureUnit,
): AmbientLaneRow[] => {
  const bucketByKey = new Map(
    (series?.buckets ?? []).map((bucket) => [String(bucket.timestamp), bucket]),
  );

  return rows.map((row) => {
    const bucket = bucketByKey.get(row.key);
    return {
      key: row.key,
      label: row.label,
      // An absolute point converts with the +32 offset; the delta beside it
      // is a span and must not.
      ambient: toDisplayTemperature(
        bucket?.ambientAvg ?? null,
        temperatureUnit,
      ),
      delta:
        bucket?.deltaAvg == null || !Number.isFinite(bucket.deltaAvg)
          ? null
          : Number.parseFloat(
              convertTemperatureDelta(bucket.deltaAvg, temperatureUnit).toFixed(
                1,
              ),
            ),
    };
  });
};

/**
 * Y-axis domain for the ambient lane, in display degrees, and the lane's
 * capability gate: null means nothing in the window recorded ambient, and
 * the lane is then not drawn at all rather than as an empty axis reading
 * "0 degC measured".
 *
 * Follows the data the way the temperature lane does rather than anchoring
 * at zero like power and fan: a room sits in a narrow band well above zero,
 * so a 0-anchored axis would flatten the few degrees of movement that are
 * the entire reason to show it.
 *
 * Signed, unlike the temperature lane's: a CPU package never reads below
 * zero on the scales this app displays, but the air around an unheated
 * room, a garage, or a winter balcony does, and clamping that minimum to 0
 * would invert the domain rather than widen it.
 */
export const computeAmbientDomain = (
  rows: readonly AmbientLaneRow[],
): [number, number] | null =>
  computeSignedTemperatureDomain(rows.map((row) => row.ambient));

/**
 * What is known about this machine's ambient temperature, from the
 * currently routed period.
 *
 * Three states rather than a boolean, for the same reason
 * [`resolveRoutedPowerCapability`] needs them:
 * - `present`: the window carries ambient readings. The lane renders.
 * - `absent`: the window recorded *something* and none of it was ambient,
 *   which is real evidence of no environmental sensor.
 * - `unknown`: the fetch has not resolved, it failed, the window recorded
 *   nothing at all, or the routed period reads a source that cannot answer
 *   the question. Nothing may be claimed either way.
 */
export type AmbientCapability = "unknown" | "present" | "absent";

/**
 * Resolve [`AmbientCapability`] for the currently routed period.
 *
 * The long-range routes always answer `unknown`, and that is a fact about
 * the source rather than a shortcut: `cooling_daily_summary` stores the
 * per-band Thermal Delta and the day's ambient coverage count but no
 * ambient temperature, so a 90-day window can neither draw the lane nor
 * prove that no sensor exists. Inheriting `present` from the short archive
 * would be worse still - the lane it would be describing is the one for
 * *this* window.
 */
export const resolveRoutedAmbientCapability = (
  route: { kind: "archive" | "dailyTrend" },
  archive: {
    ambientSeries: AmbientArchiveSeries | null;
    cpuSeries: ArchiveTimelineSeries;
    hasLoaded: boolean;
    /** The CPU-side fetch failed, so the whole window is unreadable. */
    hasError: boolean;
    /**
     * Only the ambient fetch failed. The lanes above still rendered, so the
     * window is readable - but nothing may be claimed about ambient.
     */
    ambientHasError: boolean;
  },
): AmbientCapability => {
  if (route.kind !== "archive") {
    return "unknown";
  }
  if (archive.hasError || archive.ambientHasError || !archive.hasLoaded) {
    return "unknown";
  }
  if (
    archive.ambientSeries?.buckets.some(
      (bucket) =>
        bucket.ambientAvg != null && Number.isFinite(bucket.ambientAvg),
    )
  ) {
    return "present";
  }
  // A window that recorded nothing at all says nothing about the machine's
  // sensors - only that the app was not running.
  return archiveWindowRecordedAnything(archive.cpuSeries)
    ? "absent"
    : "unknown";
};

/**
 * Sensor Source Labels the panel beside the timeline may name.
 *
 * Only `present` licenses naming a source. `absent` and `unknown`
 * deliberately read the same here - both leave ambient unmentioned, which
 * under-claims rather than telling a user whose sensor is simply still
 * loading that their window has none.
 */
export const namedAmbientSources = (
  capability: AmbientCapability,
  series: AmbientArchiveSeries | null,
): readonly string[] =>
  capability === "present" ? (series?.sources ?? []) : [];
