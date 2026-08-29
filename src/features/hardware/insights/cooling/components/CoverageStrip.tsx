import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import type { CoolingDailyTrendPoint } from "@/rspc/bindings";
import { buildCoverageCells } from "../utils/coverageStrip";

/**
 * Zone (4): a self-drawn day-by-day coverage strip (one cell per day,
 * following the lightweight-SVG approach in `Sparkline.tsx` rather than
 * pulling in a charting library for a strip of rectangles). Only shown at
 * 90d/1y - at 24h/7d/30d, gaps are already visible directly in the archive
 * charts above.
 *
 * `points: null` means the trend has not been established yet (idle or
 * in flight) and renders a skeleton; `hasError` renders a load-failure
 * line. Neither may be conflated with a genuinely empty period - a
 * failed or pending fetch is not evidence of absent history.
 */
export const CoverageStrip = ({
  points,
  days,
  hasError = false,
}: {
  points: CoolingDailyTrendPoint[] | null;
  days: 90 | 365;
  hasError?: boolean;
}) => {
  const { t } = useTranslation();
  const cells = useMemo(
    () => buildCoverageCells(points ?? [], days),
    [points, days],
  );

  const hasAnyCoverage = cells.some((cell) => cell.coverageRatio > 0);

  return (
    <section
      className="rounded-2xl bg-card p-4"
      data-testid="cooling-coverage-strip"
    >
      <h3 className="mb-2 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
        {t("pages.insights.cooling.coverage.title")}
      </h3>
      {hasError ? (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.coverage.loadFailed")}
        </p>
      ) : points == null ? (
        <Skeleton
          aria-busy="true"
          className="h-6 w-full"
          data-testid="cooling-coverage-strip-loading"
        />
      ) : hasAnyCoverage ? (
        <svg
          className="h-6 w-full"
          viewBox={`0 0 ${cells.length} 1`}
          preserveAspectRatio="none"
          role="img"
          aria-label={t("pages.insights.cooling.coverage.title")}
        >
          {cells.map((cell, index) => (
            <rect
              key={cell.date}
              x={index}
              y={0}
              width={1}
              height={1}
              className={cell.coverageRatio > 0 ? "fill-primary" : "fill-muted"}
              opacity={
                cell.coverageRatio > 0 ? Math.max(cell.coverageRatio, 0.25) : 1
              }
            >
              <title>{`${cell.date}: ${Math.round(cell.coverageRatio * 100)}%`}</title>
            </rect>
          ))}
        </svg>
      ) : (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.noDataForPeriod")}
        </p>
      )}
    </section>
  );
};
