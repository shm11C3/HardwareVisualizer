import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import type { ShowDataType } from "@/types/chart";
import type { ChartDataType } from "@/types/hardwareDataType";
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
  const { settings } = useSettingsAtom();

  const { labels, chartData } = useInsightChart({
    type: "cpu_avg",
    endAt: new Date(),
    period: 180,
  });

  const chartConfig: Record<ChartDataType, { label: string; color: string }> = {
    cpu: {
      label: "CPU",
      color: `rgb(${settings.lineGraphColor.cpu})`,
    },
    memory: {
      label: "RAM",
      color: `rgb(${settings.lineGraphColor.memory})`,
    },
    gpu: {
      label: "GPU",
      color: `rgb(${settings.lineGraphColor.gpu})`,
    },
  } satisfies ChartConfig;

  return (
    <div>
      <SingleLineChart
        labels={labels}
        chartData={chartData}
        dataType={dataType}
        chartConfig={chartConfig}
        border={false}
        size="xl"
        lineGraphMix={false}
        lineGraphShowScale={true}
        lineGraphShowTooltip={true}
        lineGraphType={settings.lineGraphType}
        lineGraphShowLegend={false}
      />
    </div>
  );
};
