import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
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

    const points = dailyTrend ?? [];
    if (points.length === 0) {
      return [];
    }
    // Anchor the daily grid to the latest summarized local day Core
    // returned - its trailing window ends yesterday in the machine's
    // local timezone, so anchoring on the frontend's own clock would
    // shift the whole grid by a day (dropping the oldest returned day
    // and appending a false empty one), and further across timezone
    // boundaries. "YYYY-MM-DD" sorts lexicographically as chronologically.
    const latestDate = points.reduce(
      (max, point) => (point.date > max ? point.date : max),
      points[0].date,
    );
    const formatter = new Intl.DateTimeFormat(
      undefined,
      dailyDateFormatOptions(route.days),
    );
    return buildDailyTimelineRows(
      points,
      route.days,
      new Date(`${latestDate}T00:00:00Z`),
      temperatureUnit,
      (isoDate) => formatter.format(new Date(`${isoDate}T00:00:00Z`)),
    );
  }, [route, series, stepMs, dailyTrend, temperatureUnit]);

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

  const hasLoadData = rows.some(
    (row) =>
      row.cpuUsage != null ||
      row.loadIdle != null ||
      row.loadLow != null ||
      row.loadMid != null ||
      row.loadHigh != null,
  );

  return (
    <section
      className="rounded-2xl bg-card p-4"
      data-testid="cooling-thermal-timeline-lane"
    >
      <h3 className="mb-3 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
        {t("pages.insights.cooling.timeline.title")}
      </h3>
      {route.kind === "dailyTrend" && dailyTrend == null ? (
        <Skeleton
          aria-busy="true"
          className="h-50 w-full"
          data-testid="cooling-timeline-loading"
        />
      ) : domain == null && !hasLoadData ? (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.noDataForPeriod")}
        </p>
      ) : (
        // `domain == null` with load data still renders: archived CPU
        // usage without a working temperature sensor is useful partial
        // data, so only the temperature lane degrades (DP-02).
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
