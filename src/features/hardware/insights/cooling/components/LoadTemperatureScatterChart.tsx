import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  CartesianGrid,
  ReferenceLine,
  ResponsiveContainer,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import type { TemperatureUnit } from "@/rspc/bindings";
import {
  cpuLoadAxisDomain,
  cpuLoadBandDividers,
  type ExplorerMedianPoint,
  type ExplorerScatterPoint,
  explorerWindowColors,
} from "../utils/loadTemperatureExplorer";
import { computeAdaptiveTemperatureDomain } from "../utils/thermalTimeline";

/**
 * The Explorer's scatter: one dot per recorded hour, two windows overlaid,
 * with the CPU-load band dividers drawn as vertical reference lines and
 * each window's per-band median temperature joined into a trend line.
 *
 * Every median plotted here was computed in Core (see
 * `cooling_load_temperature_explorer`); this component only positions and
 * unit-converts them. The recent window is drawn last so it reads on top
 * of the baseline cloud.
 */
export const LoadTemperatureScatterChart = ({
  baselinePoints,
  recentPoints,
  baselineMedians,
  recentMedians,
  temperatureUnit,
}: {
  baselinePoints: ExplorerScatterPoint[];
  recentPoints: ExplorerScatterPoint[];
  baselineMedians: ExplorerMedianPoint[];
  recentMedians: ExplorerMedianPoint[];
  temperatureUnit: TemperatureUnit;
}) => {
  const { t } = useTranslation();
  const unitSuffix = temperatureUnit === "C" ? "°C" : "°F";

  const domain = useMemo(
    () =>
      computeAdaptiveTemperatureDomain([
        ...baselinePoints.map((point) => point.y),
        ...recentPoints.map((point) => point.y),
        ...baselineMedians.map((point) => point.y),
        ...recentMedians.map((point) => point.y),
      ]),
    [baselinePoints, recentPoints, baselineMedians, recentMedians],
  );

  if (domain == null) {
    return (
      <p className="text-muted-foreground text-sm">
        {t("pages.insights.noDataForPeriod")}
      </p>
    );
  }

  return (
    <div data-testid="cooling-explorer-scatter">
      <div className="mb-2 flex flex-wrap items-center gap-4 text-muted-foreground text-xs">
        <span className="flex items-center gap-1.5">
          <span
            aria-hidden
            className="h-2.5 w-2.5 rounded-full"
            style={{ backgroundColor: explorerWindowColors.baseline }}
          />
          {t("pages.insights.cooling.explorer.legend.baseline")}
        </span>
        <span className="flex items-center gap-1.5">
          <span
            aria-hidden
            className="h-2.5 w-2.5 rounded-full"
            style={{ backgroundColor: explorerWindowColors.recent }}
          />
          {t("pages.insights.cooling.explorer.legend.recent")}
        </span>
        <span>{t("pages.insights.cooling.explorer.legend.median")}</span>
      </div>

      <ResponsiveContainer width="100%" height={320}>
        <ScatterChart margin={{ top: 8, right: 16, bottom: 24, left: 0 }}>
          <CartesianGrid strokeDasharray="3 3" />
          <XAxis
            type="number"
            dataKey="x"
            domain={cpuLoadAxisDomain as unknown as [number, number]}
            ticks={[0, ...cpuLoadBandDividers, 100]}
            tickFormatter={(value: number) => `${value}`}
            label={{
              value: t("pages.insights.cooling.explorer.loadAxis"),
              position: "insideBottom",
              offset: -12,
            }}
          />
          <YAxis
            type="number"
            dataKey="y"
            domain={domain}
            width={48}
            label={{
              value: t("pages.insights.cooling.explorer.temperatureAxis", {
                unit: unitSuffix,
              }),
              angle: -90,
              position: "insideLeft",
            }}
          />
          {/* The band edges are Core's (`CpuLoadBand::classify`); the
              chart draws them rather than defining them. */}
          {cpuLoadBandDividers.map((divider) => (
            <ReferenceLine
              key={divider}
              x={divider}
              stroke="hsl(var(--muted-foreground))"
              strokeDasharray="2 4"
              strokeOpacity={0.6}
            />
          ))}
          <Tooltip
            cursor={{ strokeDasharray: "3 3" }}
            formatter={(value, name) => {
              const numeric = Number(value);
              return name === "y"
                ? [
                    `${numeric.toFixed(1)}${unitSuffix}`,
                    t("pages.insights.cooling.explorer.temperatureAxis", {
                      unit: unitSuffix,
                    }),
                  ]
                : [
                    `${numeric.toFixed(1)} %`,
                    t("pages.insights.cooling.explorer.loadAxis"),
                  ];
            }}
          />
          <Scatter
            name={t("pages.insights.cooling.explorer.legend.baseline")}
            data={baselinePoints}
            fill={explorerWindowColors.baseline}
            fillOpacity={0.45}
          />
          <Scatter
            name={t("pages.insights.cooling.explorer.legend.recent")}
            data={recentPoints}
            fill={explorerWindowColors.recent}
            fillOpacity={0.55}
          />
          {/* `line` joins the four band medians into the trend line; the
              points themselves stay visible as its vertices. */}
          <Scatter
            name={t("pages.insights.cooling.explorer.legend.median")}
            data={baselineMedians}
            fill={explorerWindowColors.baseline}
            line={{ stroke: explorerWindowColors.baseline, strokeWidth: 2 }}
            lineJointType="linear"
          />
          <Scatter
            name={t("pages.insights.cooling.explorer.legend.median")}
            data={recentMedians}
            fill={explorerWindowColors.recent}
            line={{ stroke: explorerWindowColors.recent, strokeWidth: 2 }}
            lineJointType="linear"
          />
        </ScatterChart>
      </ResponsiveContainer>
    </div>
  );
};
