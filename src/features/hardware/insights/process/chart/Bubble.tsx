import { useTranslation } from "react-i18next";
import {
  CartesianGrid,
  getNiceTickValues,
  ResponsiveContainer,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
  ZAxis,
} from "recharts";
import { bubbleChartColor } from "@/features/hardware/consts/chart";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";
import { CustomTooltip } from "./components/CustomTooltip";
import { getProcessBubbleData } from "./getProcessBubbleData";
import { useScatterChartZoom } from "./hooks/useScatterChartZoom";

export const ProcessBubbleChart = ({
  processStats,
}: {
  processStats: ProcessStat[] | null;
  loading: boolean;
}) => {
  const { t } = useTranslation();

  const { allData, data, omittedCount, maxX, maxZ } = getProcessBubbleData(
    processStats ?? [],
  );
  // Match Recharts' automatic X ticks before limiting the visible points.
  const fullXTicks = getNiceTickValues([0, maxX], 5, true, "adaptive");

  const {
    containerRef,
    xDomain,
    yDomain,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
    cursorStyle,
  } = useScatterChartZoom(allData);
  const automaticXDomain = xDomain[1] === "auto" && allData.length > 0;

  return (
    <>
      <section
        ref={containerRef}
        className="relative select-none"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        aria-label="Process Bubble Chart"
      >
        <ResponsiveContainer width="100%" height={500}>
          <ScatterChart
            margin={{ top: 20, right: 20, bottom: 40, left: 20 }}
            style={cursorStyle}
          >
            <CartesianGrid />
            <XAxis
              dataKey="x"
              name={`${t("shared.execTime")} ${t("shared.time.minutes")}`}
              type="number"
              domain={
                automaticXDomain
                  ? [0, fullXTicks[fullXTicks.length - 1]]
                  : xDomain
              }
              {...(automaticXDomain ? { ticks: fullXTicks } : {})}
              label={{
                value: `${t("shared.execTime")} (${t("shared.time.minutes")})`,
                position: "insideBottom",
                offset: -10,
              }}
              allowDataOverflow={true}
            />
            <YAxis
              dataKey="y"
              name={`${t("shared.cpuUsage")} (%)`}
              type="number"
              domain={yDomain}
              label={{
                value: `${t("shared.cpuUsage")} (%)`,
                angle: -90,
                position: "insideLeft",
              }}
              allowDataOverflow={true}
            />
            <ZAxis
              dataKey="z"
              domain={[0, maxZ]}
              range={[60, 400]}
              name="Cumulative CPU load"
            />
            <Tooltip
              cursor={{ strokeDasharray: "3 3" }}
              formatter={(value, name) => {
                switch (name) {
                  case "x":
                    return [
                      `${(value as number).toFixed(1)} ${t("shared.time.minutes")}`,
                      t("shared.execTime"),
                    ];
                  case "y":
                    return [`${(value as number).toFixed(1)} %`, "CPU"];
                  default:
                    return value;
                }
              }}
              content={<CustomTooltip />}
            />
            <Scatter name="process" data={data} fill={bubbleChartColor} />
          </ScatterChart>
        </ResponsiveContainer>
      </section>
      {omittedCount > 0 && (
        <p className="px-5 pb-3 text-muted-foreground text-sm">
          {t("pages.insights.process.chartSummary", {
            displayed: data.length,
            omitted: omittedCount,
          })}
        </p>
      )}
    </>
  );
};
