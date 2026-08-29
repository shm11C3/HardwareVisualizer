import type { ArchivePeriod } from "@/features/hardware/insights/utils/archivePeriod";
import type { CoolingInsightPeriod } from "../types";

/**
 * Where a selected Cooling Insight period gets its data from:
 * - 24h/7d/30d have direct `ArchivePeriod` equivalents, so they reuse the
 *   existing `getDataArchiveSeries` bucket query (via `useInsightChart`).
 * - 90d/1y have no archive-bucket equivalent (buckets that wide would read
 *   an enormous row count for a single chart); Core precomputes daily
 *   summaries for exactly this range via `getCoolingTrend`.
 */
/** The archive bucket widths the Cooling tab's 24h/7d/30d periods map to. */
export type CoolingArchivePeriod = Extract<ArchivePeriod, 1440 | 10080 | 43200>;

export type CoolingPeriodRoute =
  | { kind: "archive"; minutes: CoolingArchivePeriod }
  | { kind: "dailyTrend"; days: 90 | 365 };

export const resolveCoolingPeriodRoute = (
  period: CoolingInsightPeriod,
): CoolingPeriodRoute => {
  switch (period) {
    case "24h":
      return { kind: "archive", minutes: 1440 };
    case "7d":
      return { kind: "archive", minutes: 10080 };
    case "30d":
      return { kind: "archive", minutes: 43200 };
    case "90d":
      return { kind: "dailyTrend", days: 90 };
    case "1y":
      return { kind: "dailyTrend", days: 365 };
  }
};
