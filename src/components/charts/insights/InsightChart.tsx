import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import type { archivePeriods } from "@/consts";
import type { ChartDataType } from "@/types/hardwareDataType";
import { useInsightChart } from "./useInsightChart";

export const InsightChart = ({
  dataType,
  period,
  type,
}: {
  dataType: "cpu" | "memory" | "gpu";
  period: (typeof archivePeriods)[number];
  type: "cpu_avg" | "cpu_max" | "cpu_min" | "ram_avg" | "ram_max" | "ram_min";
}) => {
  const { settings } = useSettingsAtom();

  const { labels, chartData } = useInsightChart({
    type: type,
    endAt: new Date(),
    period: period,
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
        size="lg"
        lineGraphMix={false}
        lineGraphShowScale={true}
        lineGraphShowTooltip={true}
        lineGraphType={settings.lineGraphType}
        lineGraphShowLegend={false}
      />
    </div>
  );
};
