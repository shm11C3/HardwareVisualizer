import { useTranslation } from "react-i18next";
import {
  CartesianGrid,
  Line,
  LineChart,
  ReferenceLine,
  ResponsiveContainer,
  XAxis,
  YAxis,
} from "recharts";
import type { TemperatureUnit } from "@/rspc/bindings";
import {
  type FitLineChart,
  fitLineColors,
  temperatureUnitSuffix,
} from "../utils/covariateComparison";
import { computeSignedTemperatureDomain } from "../utils/thermalTimeline";

/**
 * ΔT against package power for the compared band: each window's
 * least-squares line drawn across the inferred power range, and nothing
 * else. Core sends the fits but no paired minutes, so there is no cloud to
 * draw - and none is invented (see `buildFitLineChart`). The baseline's
 * median power is marked because that is where the lead sentence reads
 * the two lines against each other.
 */
export const CovariateFitChart = ({
  chart,
  temperatureUnit,
}: {
  chart: FitLineChart;
  temperatureUnit: TemperatureUnit;
}) => {
  const { t } = useTranslation();
  const unitSuffix = temperatureUnitSuffix(temperatureUnit);
  // Signed on purpose: a ΔT line can cross zero, and a fit's intercept
  // routinely sits below it.
  const domain = computeSignedTemperatureDomain(
    chart.rows.flatMap((row) => [row.baseline, row.recent]),
  );

  return (
    <div data-testid="cooling-covariate-chart">
      <ResponsiveContainer width="100%" height={260}>
        <LineChart
          data={chart.rows}
          margin={{ top: 8, right: 16, bottom: 24, left: 0 }}
        >
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis
            type="number"
            dataKey="x"
            domain={chart.domain}
            tickFormatter={(value: number) => `${value} W`}
            label={{
              value: t(
                "pages.insights.cooling.covariateComparison.chart.powerAxis",
              ),
              position: "insideBottom",
              offset: -12,
            }}
          />
          {/* No rotated axis label: at this height it clips, and the
              chart's title already names the reading and its unit. */}
          <YAxis
            type="number"
            {...(domain == null ? {} : { domain })}
            width={48}
            tickFormatter={(value: number) => `${value}${unitSuffix}`}
          />
          <ReferenceLine
            x={chart.anchorPower}
            stroke="var(--muted-foreground)"
            strokeDasharray="2 4"
            strokeOpacity={0.6}
          />
          {chart.baselineSlope != null && (
            <Line
              type="linear"
              dataKey="baseline"
              stroke={fitLineColors.baseline}
              strokeWidth={2}
              strokeDasharray="6 4"
              dot={false}
              isAnimationActive={false}
            />
          )}
          {chart.recentSlope != null && (
            <Line
              type="linear"
              dataKey="recent"
              stroke={fitLineColors.recent}
              strokeWidth={2}
              dot={false}
              isAnimationActive={false}
            />
          )}
        </LineChart>
      </ResponsiveContainer>

      <div className="mt-1 flex flex-wrap items-center gap-4 text-muted-foreground text-xs">
        {chart.baselineSlope != null && (
          <span className="flex items-center gap-1.5">
            <span
              aria-hidden
              className="h-0.5 w-4 border-t-2 border-dashed"
              style={{ borderColor: fitLineColors.baseline }}
            />
            {t(
              "pages.insights.cooling.covariateComparison.chart.legend.baseline",
              { slope: chart.baselineSlope },
            )}
          </span>
        )}
        {chart.recentSlope != null && (
          <span className="flex items-center gap-1.5">
            <span
              aria-hidden
              className="h-0.5 w-4"
              style={{ backgroundColor: fitLineColors.recent }}
            />
            {t(
              "pages.insights.cooling.covariateComparison.chart.legend.recent",
              { slope: chart.recentSlope },
            )}
          </span>
        )}
        <span className="flex items-center gap-1.5">
          <span
            aria-hidden
            className="h-3 border-l border-dashed"
            style={{ borderColor: "var(--muted-foreground)" }}
          />
          {t("pages.insights.cooling.covariateComparison.chart.anchorPower", {
            power: chart.anchorPower.toFixed(1),
          })}
        </span>
      </div>
    </div>
  );
};
