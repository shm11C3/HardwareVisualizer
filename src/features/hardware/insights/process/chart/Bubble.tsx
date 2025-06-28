import { useTranslation } from "react-i18next";
import {
  CartesianGrid,
  ResponsiveContainer,
  Scatter,
  ScatterChart,
  Tooltip,
  XAxis,
  YAxis,
  ZAxis,
} from "recharts";
import { bubbleChartColor } from "@/features/hardware/consts/chart";
import type { ProcessStat } from "../types/processStats";
import { CustomTooltip } from "./components/CustomTooltip";
import { useScatterChartZoom } from "./hooks/useScatterChartZoom";

export const ProcessBubbleChart = ({
  processStats,
}: {
  processStats: ProcessStat[] | null;
  loading: boolean;
}) => {
  const { t } = useTranslation();

  const chartData =
    processStats == null
      ? []
      : processStats.map((d) => {
          const rawZ = (d.avg_cpu_usage ?? 0) * d.total_execution_sec;

          return {
            x: d.total_execution_sec / 60, // 実行時間（分）
            y: d.avg_cpu_usage, // CPU使用率
            z: Math.sqrt(rawZ),
            name: d.process_name,
            pid: d.pid,
            zRaw: rawZ,
            ram: d.avg_memory_usage,
          };
        });

  const {
    containerRef,
    xDomain,
    yDomain,
    handleMouseDown,
    handleMouseMove,
    handleMouseUp,
    cursorStyle,
  } = useScatterChartZoom(chartData);

  return (
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
            domain={xDomain}
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
          <ZAxis dataKey="z" range={[60, 400]} name="Cumulative CPU load" />
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
          <Scatter name="process" data={chartData} fill={bubbleChartColor} />
        </ScatterChart>
      </ResponsiveContainer>
    </section>
  );
};
