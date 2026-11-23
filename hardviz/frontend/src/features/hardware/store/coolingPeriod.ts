import { atom } from "jotai";
import type { CoolingInsightPeriod } from "@/features/hardware/insights/cooling/types";
import {
  type CoolingPeriodRoute,
  resolveCoolingPeriodRoute,
} from "@/features/hardware/insights/cooling/utils/coolingPeriodRoute";

/**
 * The Cooling tab's single period selector. Hydrated from Tauri Store by
 * `useCoolingInsightPeriodPersistence`; until `coolingInsightPeriodHydratedAtom`
 * is true this holds the default, which nothing may render or fetch for.
 */
export const coolingInsightPeriodAtom = atom<CoolingInsightPeriod>("24h");

/** False until the stored period has been read once for this window. */
export const coolingInsightPeriodHydratedAtom = atom(false);

/** Where the selected period gets its data from. */
export const coolingPeriodRouteAtom = atom<CoolingPeriodRoute>((get) =>
  resolveCoolingPeriodRoute(get(coolingInsightPeriodAtom)),
);

/** `days` of the daily rollup, or null when the period routes to the archive. */
export const coolingDailyTrendDaysAtom = atom((get) => {
  const route = get(coolingPeriodRouteAtom);
  return route.kind === "dailyTrend" ? route.days : null;
});

/** Archive window in minutes, or null when the period routes to the rollup. */
export const coolingArchiveMinutesAtom = atom((get) => {
  const route = get(coolingPeriodRouteAtom);
  return route.kind === "archive" ? route.minutes : null;
});
