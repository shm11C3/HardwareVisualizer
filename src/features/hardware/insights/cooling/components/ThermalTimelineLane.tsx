import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type {
  CoolingBaselineState,
  CoolingDailyTrendPoint,
} from "@/rspc/bindings";
import { useCoolingArchiveTimeline } from "../hooks/useCoolingArchiveTimeline";
import type { CoolingPeriodRoute } from "../utils/coolingPeriodRoute";
import {
  buildArchiveTimelineRows,
  buildDailyTimelineRows,
  collectTemperatureDomainValues,
  computeAdaptiveTemperatureDomain,
  resolveBaselineBand,
  type ThermalTimelineRow,
} from "../utils/thermalTimeline";
import { type LoadLaneMode, TimelineLanes } from "./TimelineLanes";

const dailyDateFormatOptions = (days: number): Intl.DateTimeFormatOptions =>
  days > 90
    ? { year: "numeric", month: "numeric", day: "2-digit" }
    : { month: "numeric", day: "2-digit" };

/** Same axis-label density `useInsightChart` uses for the archive periods. */
const archiveDateFormatOptions = (
  minutes: number,
): Intl.DateTimeFormatOptions => {
  const options: Intl.DateTimeFormatOptions = {};
  if (minutes >= 1440) {
    options.year = "numeric";
  }
  if (minutes >= 180) {
    options.month = "numeric";
    options.day = "2-digit";
  }
  if (minutes < 10080) {
    options.hour = "2-digit";
    options.minute = "2-digit";
  }
  return options;
};

/**
 * Zone (2) of the Cooling Insight layout: the synchronized thermal timeline.
 *
 * Both periods feed the same two lanes, from the two sources that can
 * actually answer for them:
 * - 24h/7d/30d read the archive buckets, so the load lane is bucket-average
 *   CPU usage.
 * - 90d/1y read the daily rollup, which stores per-band sample minutes but
 *   no intra-day usage series, so the load lane shows how each day was split
 *   across the load bands instead of inventing a daily average.
 */
export const ThermalTimelineLane = ({
  route,
  baseline,
  dailyTrend,
}: {
  route: CoolingPeriodRoute;
  baseline: CoolingBaselineState | null;
  dailyTrend: CoolingDailyTrendPoint[] | null;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const temperatureUnit = settings.temperatureUnit;

  const { series, stepMs } = useCoolingArchiveTimeline(
    route.kind === "archive" ? route.minutes : null,
  );

  // The daily window ends today; keying the reference date on the UTC day
  // keeps the rows stable across renders within the same day.
  const todayKey = new Date().toISOString().slice(0, 10);

  const rows = useMemo<ThermalTimelineRow[]>(() => {
    if (route.kind === "archive") {
      const formatter = new Intl.DateTimeFormat(
        undefined,
        archiveDateFormatOptions(route.minutes),
      );
      return buildArchiveTimelineRows(
        series,
        stepMs,
        temperatureUnit,
        (timestamp) => formatter.format(new Date(timestamp)),
      );
    }

    const formatter = new Intl.DateTimeFormat(
      undefined,
      dailyDateFormatOptions(route.days),
    );
    return buildDailyTimelineRows(
      dailyTrend ?? [],
      route.days,
      new Date(`${todayKey}T00:00:00Z`),
      temperatureUnit,
      (isoDate) => formatter.format(new Date(`${isoDate}T00:00:00Z`)),
    );
  }, [route, series, stepMs, dailyTrend, temperatureUnit, todayKey]);

  const baselineBand = useMemo(
    () => resolveBaselineBand(baseline, temperatureUnit),
    [baseline, temperatureUnit],
  );

  const domain = useMemo(
    () =>
      computeAdaptiveTemperatureDomain(
        collectTemperatureDomainValues(
          rows,
          baselineBand == null ? [] : [baselineBand.lower, baselineBand.upper],
        ),
      ),
    [rows, baselineBand],
  );

  const loadMode: LoadLaneMode =
    route.kind === "archive" ? "usage" : "composition";

  return (
    <section
      className="rounded-2xl bg-card p-4"
      data-testid="cooling-thermal-timeline-lane"
    >
      <h3 className="mb-3 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
        {t("pages.insights.cooling.timeline.title")}
      </h3>
      {domain == null ? (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.noDataForPeriod")}
        </p>
      ) : (
        <TimelineLanes
          rows={rows}
          domain={domain}
          baseline={baselineBand}
          loadMode={loadMode}
          temperatureUnit={temperatureUnit}
        />
      )}
    </section>
  );
};
