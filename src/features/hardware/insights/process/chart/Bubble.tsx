import { Skeleton } from "@/components/ui/skeleton";
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
import { CustomTooltip } from "./components/CustomTooltip";

export const ProcessBubbleChart = ({
  processStats,
  loading,
}: {
  processStats: ProcessStat[] | null;
  loading: boolean;
}) => {
  const { t } = useTranslation();

  if (loading || processStats == null) {
    return <Skeleton className="w-full h-[500px]" />;
  }

  const chartData = processStats.map((d) => {
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
        <Scatter name="process" data={chartData} fill="#8884d8" />
      </ScatterChart>
    </ResponsiveContainer>
  );
};
