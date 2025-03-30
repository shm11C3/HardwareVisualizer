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

export function ProcessBubbleChart({
  processStats,
  loading,
}: {
  processStats: ProcessStat[] | null;
  loading: boolean;
}) {
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
          name="実行時間（分）"
          type="number"
          label={{
            value: "実行時間（分）",
            position: "insideBottom",
            offset: -10,
          }}
        />
        <YAxis
          dataKey="y"
          name="CPU使用率（%）"
          type="number"
          domain={[0, 100]}
          label={{
            value: "CPU使用率（%）",
            angle: -90,
            position: "insideLeft",
          }}
        />
        <ZAxis
          dataKey="z"
          range={[60, 400]} // バブルサイズ（調整可能）
          name="メモリ使用量（MB）"
        />
        <Tooltip
          cursor={{ strokeDasharray: "3 3" }}
          formatter={(value, name) => {
            switch (name) {
              case "x":
                return [`${(value as number).toFixed(1)} 分`, "実行時間"];
              case "y":
                return [`${(value as number).toFixed(1)} %`, "CPU"];
              case "z":
                return [`${(value as number).toFixed(1)} MB`, "メモリ"];
              default:
                return value;
            }
          }}
          content={<CustomTooltip />}
        />
        <Scatter name="プロセス" data={chartData} fill="#8884d8" />
      </ScatterChart>
    </ResponsiveContainer>
  );
}
