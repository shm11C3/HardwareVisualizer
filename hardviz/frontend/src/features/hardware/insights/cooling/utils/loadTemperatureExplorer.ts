import type {
  CoolingBandMedianDelta,
  CoolingExplorerWindow,
  CoolingLoadBand,
  TemperatureUnit,
} from "@/rspc/bindings";
import { convertTemperatureDelta } from "./temperatureUnit";
import { toDisplayTemperature } from "./thermalTimeline";

/**
 * Trailing-window lengths the Explorer offers. Both ends match Core's
 * `COOLING_EXPLORER_MIN_RECENT_DAYS` / `COOLING_EXPLORER_MAX_RECENT_DAYS`
 * clamp, so no preset can ask for a window Core would silently narrow.
 */
export const explorerRecentDayPresets = [7, 28, 90] as const;

export type ExplorerRecentDays = (typeof explorerRecentDayPresets)[number];

/**
 * Default trailing window: long enough to fill every load band on a normal
 * machine, short enough that the scatter still reads as "recently", rather
 * than blending a quarter of a year into one cloud.
 */
export const defaultExplorerRecentDays: ExplorerRecentDays = 28;

export const isExplorerRecentDays = (
  value: unknown,
): value is ExplorerRecentDays =>
  typeof value === "number" &&
  (explorerRecentDayPresets as readonly number[]).includes(value);

/**
 * Upper CPU-usage bound of each band except the open-ended top one, in
 * percent. These are the dividers the scatter draws, and they mirror
 * `CpuLoadBand::classify` in Core - the chart must not invent its own
 * band edges.
 */
export const cpuLoadBandDividers = [10, 30, 60] as const;

/** X-axis extent of the scatter, in percent CPU usage. */
export const cpuLoadAxisDomain: readonly [number, number] = [0, 100];

/**
 * The two windows' colors, shared by every part of the Explorer that
 * distinguishes them (scatter, median trend lines, period minimap). One
 * definition so "baseline" and "recent" never read as different pairs of
 * colors within the same panel.
 */
export const explorerWindowColors = {
  baseline: "hsl(var(--chart-3))",
  recent: "hsl(var(--chart-1))",
} as const;

/**
 * Horizontal position each band's median is plotted at: the middle of the
 * band's usage range, with the open-ended top band centered between its
 * lower divider and the axis maximum. A median is a summary of the whole
 * band, so it belongs at the band's center rather than at either edge.
 */
export const bandMedianPositions: Record<CoolingLoadBand, number> = {
  idle: cpuLoadBandDividers[0] / 2,
  low: (cpuLoadBandDividers[0] + cpuLoadBandDividers[1]) / 2,
  mid: (cpuLoadBandDividers[1] + cpuLoadBandDividers[2]) / 2,
  high: (cpuLoadBandDividers[2] + cpuLoadAxisDomain[1]) / 2,
};

/** One scatter point, in display units. */
export type ExplorerScatterPoint = {
  hourStart: string;
  /** CPU usage, in percent. */
  x: number;
  /** CPU temperature, in the caller's display unit. */
  y: number;
  sampleMinutes: number;
};

/**
 * Convert one window's hourly pairs into scatter points. A point whose
 * temperature cannot be expressed in the display unit is dropped rather
 * than plotted at a substituted value.
 */
export const buildExplorerScatterPoints = (
  window: CoolingExplorerWindow | null,
  temperatureUnit: TemperatureUnit,
): ExplorerScatterPoint[] => {
  if (window == null) {
    return [];
  }

  return window.points.flatMap((point) => {
    const y = toDisplayTemperature(point.cpuTemperatureAvg, temperatureUnit);
    return y == null
      ? []
      : [
          {
            hourStart: point.hourStart,
            x: point.cpuUsageAvg,
            y,
            sampleMinutes: point.sampleMinutes,
          },
        ];
  });
};

/** One point of a window's per-band median trend line, in display units. */
export type ExplorerMedianPoint = {
  band: CoolingLoadBand;
  x: number;
  y: number;
};

/**
 * The per-band median trend line for one window, left to right across the
 * bands. Values come straight from Core - this only positions them and
 * converts the display unit. A band with no median is absent, so the line
 * spans the bands that were actually observed instead of dipping to zero.
 */
export const buildExplorerMedianTrend = (
  bandDeltas: readonly CoolingBandMedianDelta[],
  side: "baseline" | "recent",
  temperatureUnit: TemperatureUnit,
): ExplorerMedianPoint[] =>
  bandDeltas.flatMap((entry) => {
    const y = toDisplayTemperature(
      entry[side].temperatureMedian,
      temperatureUnit,
    );
    return y == null
      ? []
      : [{ band: entry.band, x: bandMedianPositions[entry.band], y }];
  });

/** One row of the per-band delta list. */
export type ExplorerBandDeltaRow =
  | {
      band: CoolingLoadBand;
      comparable: false;
      baselinePointCount: number;
      recentPointCount: number;
    }
  | {
      band: CoolingLoadBand;
      comparable: true;
      baseline: number;
      recent: number;
      delta: number;
      baselinePointCount: number;
      recentPointCount: number;
    };

/**
 * Convert Core's per-band deltas into display-ready rows.
 *
 * `comparable` is Core's own verdict; a band is also folded into the
 * non-comparable shape when a value it needs is unexpectedly missing,
 * since a delta row cannot be drawn without both medians. The point
 * counts are carried either way so a non-comparable row can still say how
 * little evidence there was.
 */
export const buildExplorerBandDeltaRows = (
  bandDeltas: readonly CoolingBandMedianDelta[],
  temperatureUnit: TemperatureUnit,
): ExplorerBandDeltaRow[] =>
  bandDeltas.map((entry) => {
    const counts = {
      band: entry.band,
      baselinePointCount: entry.baseline.pointCount,
      recentPointCount: entry.recent.pointCount,
    };

    if (
      !entry.comparable ||
      entry.delta == null ||
      entry.baseline.temperatureMedian == null ||
      entry.recent.temperatureMedian == null
    ) {
      return { ...counts, comparable: false };
    }

    const baseline = toDisplayTemperature(
      entry.baseline.temperatureMedian,
      temperatureUnit,
    );
    const recent = toDisplayTemperature(
      entry.recent.temperatureMedian,
      temperatureUnit,
    );
    if (baseline == null || recent == null) {
      return { ...counts, comparable: false };
    }

    return {
      ...counts,
      comparable: true,
      baseline,
      recent,
      delta: convertTemperatureDelta(entry.delta, temperatureUnit),
    };
  });

/** One window's position on the read-only period minimap, in percent. */
export type ExplorerMinimapSegment = {
  kind: "baseline" | "recent";
  startDate: string;
  endDate: string;
  /** Left edge, 0-100, within the minimap's overall span. */
  offsetPercent: number;
  /** Width, 0-100. Never zero, so a single-day window stays visible. */
  widthPercent: number;
};

/** Smallest width a window occupies on the minimap, in percent. */
const MINIMAP_MIN_WIDTH_PERCENT = 1.5;

const MS_PER_DAY = 24 * 60 * 60 * 1000;

const parseIsoDate = (isoDate: string): number | null => {
  // Anchored at UTC midnight so the minimap's arithmetic is unaffected by
  // the viewer's offset; only the span between dates matters here.
  const parsed = Date.parse(`${isoDate}T00:00:00Z`);
  return Number.isNaN(parsed) ? null : parsed;
};

/**
 * Lay both windows out on a shared timeline running from the earlier
 * window's start to the later window's end.
 *
 * Read-only by design: the Explorer offers fixed trailing-window presets
 * rather than a draggable brush, so this only has to show *where* the two
 * compared periods sit relative to each other and how far apart they are.
 *
 * Returns an empty array when either window's dates cannot be read, so
 * the caller omits the minimap instead of drawing a misleading one.
 */
export const buildExplorerMinimapSegments = (
  baseline: Pick<CoolingExplorerWindow, "startDate" | "endDate">,
  recent: Pick<CoolingExplorerWindow, "startDate" | "endDate">,
): ExplorerMinimapSegment[] => {
  const bounds = [baseline, recent].map((window) => ({
    start: parseIsoDate(window.startDate),
    end: parseIsoDate(window.endDate),
  }));
  if (bounds.some(({ start, end }) => start == null || end == null)) {
    return [];
  }

  const starts = bounds.map(({ start }) => start as number);
  const ends = bounds.map(({ end }) => end as number);
  const spanStart = Math.min(...starts);
  const spanEnd = Math.max(...ends);
  // Both window lengths and the overall span are inclusive of their end
  // day: a window is "2026-01-01 through 2026-01-07", seven days, not the
  // six-day gap between those two midnights. Measuring exclusively would
  // give a single-day window zero width and every other window one day
  // less than it covers.
  const span = spanEnd - spanStart + MS_PER_DAY;

  return [
    { kind: "baseline" as const, window: baseline, index: 0 },
    { kind: "recent" as const, window: recent, index: 1 },
  ].map(({ kind, window, index }) => {
    if (span <= 0) {
      return {
        kind,
        startDate: window.startDate,
        endDate: window.endDate,
        offsetPercent: 0,
        widthPercent: 100,
      };
    }

    const offsetPercent = ((starts[index] - spanStart) / span) * 100;
    const widthPercent = Math.max(
      MINIMAP_MIN_WIDTH_PERCENT,
      ((ends[index] - starts[index] + MS_PER_DAY) / span) * 100,
    );

    return {
      kind,
      startDate: window.startDate,
      endDate: window.endDate,
      offsetPercent: Math.min(offsetPercent, 100 - widthPercent),
      widthPercent,
    };
  });
};
