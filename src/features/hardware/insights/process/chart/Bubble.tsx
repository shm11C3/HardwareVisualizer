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
import type { ProcessStat } from "../types/processStats";
import { CustomTooltip } from "./components/CutstomToolTip";

export const ProcessBubbleChart = ({
  processStats,
  loading,
}: {
  processStats: ProcessStat[] | null;
  loading: boolean;
}) => {
  const { t } = useTranslation();

  if (loading || processStats == null) {
    return <></>;
  }

  const chartData = processStats.map((d) => ({
    x: d.total_execution_sec / 60, // 実行時間（分）
    y: d.avg_cpu_usage, // CPU使用率
    z: Math.max(d.avg_memory_usage, 10), // 最低10にして可視化
    name: d.process_name,
    pid: d.pid,
  }));

  return (
    <ResponsiveContainer width="100%" height={500}>
      <ScatterChart margin={{ top: 20, right: 20, bottom: 40, left: 20 }}>
        <CartesianGrid />
        <XAxis
          dataKey="x"
          name={`${t("shared.execTime")} ${t("shared.time.minutes")}`}
          type="number"
          label={{
            value: `${t("shared.execTime")} (${t("shared.time.minutes")})`,
            position: "insideBottom",
            offset: -10,
          }}
        />
        <YAxis
          dataKey="y"
          name={`${t("shared.cpuUsage")} (%)`}
          type="number"
          domain={[0, 100]}
          label={{
            value: `${t("shared.cpuUsage")} (%)`,
            angle: -90,
            position: "insideLeft",
          }}
        />
        <ZAxis
          dataKey="z"
          range={[60, 400]} // バブルサイズ（調整可能）
          name={`${t("shared.memoryUsageValue")} (%)`}
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
              case "z":
                return [`${(value as number).toFixed(1)} MB`, "RAM"];
              default:
                return value;
            }
          }}
          content={<CustomTooltip />}
        />
        <Scatter name="process" data={chartData} fill="#8884d8" />
      </ScatterChart>
    </ResponsiveContainer>
  );
};
