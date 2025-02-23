import { LineChartComponent } from "@/components/charts/LineChart";
import type { ShowDataType } from "@/types/chart";
import { useInsightChart } from "./useInsightChart";

const ShowDataType2ChartType: Record<ShowDataType, "cpu" | "memory" | "gpu"> = {
  cpu_avg: "cpu",
  cpu_max: "cpu",
  cpu_min: "cpu",
  ram_avg: "memory",
  ram_max: "memory",
  ram_min: "memory",
};

export const InsightChart = ({
  dataType,
}: { dataType: "cpu" | "memory" | "gpu" }) => {
  const { labels, chartData } = useInsightChart({
    type: "cpu_avg",
    endAt: new Date(),
    period: 180,
  });

  return (
    <div>
      <LineChartComponent
        labels={labels}
        chartData={chartData}
        dataType={dataType}
        size="xl"
        lineGraphMix={false}
      />
    </div>
  );
};
