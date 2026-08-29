import type { CoolingDailyTrendPoint } from "@/rspc/bindings";

export type CoverageCell = {
  date: string;
  /** 0 (no recorded minutes) to 1 (a full day recorded). */
  coverageRatio: number;
};

const MINUTES_PER_DAY = 1440;

const toDateKey = (date: Date): string => date.toISOString().slice(0, 10);

/**
 * Build one cell per day in the trailing `days`-day window ending on
 * `referenceDate` (inclusive), for the coverage strip shown at 90d/1y.
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
  referenceDate: Date,
): CoverageCell[] => {
  const coverageByDate = new Map(
    points.map((point) => [point.date, point.coverageMinutes]),
  );

  const cells: CoverageCell[] = [];
  for (let offset = days - 1; offset >= 0; offset--) {
    const cellDate = new Date(referenceDate);
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
