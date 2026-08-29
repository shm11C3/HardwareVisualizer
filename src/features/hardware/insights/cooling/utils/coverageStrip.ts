import type { CoolingDailyTrendPoint } from "@/rspc/bindings";

export type CoverageCell = {
  date: string;
  /** 0 (no recorded minutes) to 1 (a full day recorded). */
  coverageRatio: number;
};

const MINUTES_PER_DAY = 1440;

const toDateKey = (date: Date): string => date.toISOString().slice(0, 10);

/**
 * Build one cell per day in the trailing `days`-day window ending on the
 * latest summarized local day the backend returned (inclusive), for the
 * coverage strip shown at 90d/1y.
 *
 * The window is anchored to the response, not to the frontend clock:
 * `getCoolingTrend` ends its trailing window on yesterday in the
 * machine's local timezone, so building cells through "today" (UTC)
 * would add a false zero-coverage cell for today and drop the oldest
 * returned day. With no points there is no anchor - the caller renders
 * its empty state instead of a fabricated all-zero strip.
 *
 * `points` follows the `CoolingDailyTrendPoint` contract: a day the rollup
 * has no row for is simply absent from the array, never a zero-filled entry.
 * This function is what turns that absence into an explicit zero-coverage
 * cell, so the strip renders a visible gap instead of silently shrinking to
 * however many rows happened to come back.
 */
export const buildCoverageCells = (
  points: readonly CoolingDailyTrendPoint[],
  days: number,
): CoverageCell[] => {
  if (points.length === 0) {
    return [];
  }

  const coverageByDate = new Map(
    points.map((point) => [point.date, point.coverageMinutes]),
  );
  // "YYYY-MM-DD" sorts lexicographically the same as chronologically.
  const latestDate = points.reduce(
    (max, point) => (point.date > max ? point.date : max),
    points[0].date,
  );
  const anchor = new Date(`${latestDate}T00:00:00Z`);

  const cells: CoverageCell[] = [];
  for (let offset = days - 1; offset >= 0; offset--) {
    const cellDate = new Date(anchor);
    cellDate.setUTCDate(cellDate.getUTCDate() - offset);
    const date = toDateKey(cellDate);
    const coverageMinutes = coverageByDate.get(date) ?? 0;

    cells.push({
      date,
      coverageRatio: Math.max(
        0,
        Math.min(coverageMinutes / MINUTES_PER_DAY, 1),
      ),
    });
  }

  return cells;
};
