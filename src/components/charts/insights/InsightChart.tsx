import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import type { archivePeriods } from "@/consts";
import type { HardwareType } from "@/rspc/bindings";
import type { ChartDataType, DataStats } from "@/types/hardwareDataType";
import { useInsightChart } from "./useInsightChart";

export const InsightChart = ({
  hardwareType,
  period,
  dataStats,
  offset,
}: {
  hardwareType: Exclude<HardwareType, "gpu">;
  period: (typeof archivePeriods)[number];
  dataStats: DataStats;
  offset: number;
}) => {
  const { settings } = useSettingsAtom();
  const { labels, chartData } = useInsightChart({
    hardwareType,
    dataStats,
    period,
    offset,
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
    <div className="w-full h-full">
      <SingleLineChart
        labels={labels}
        chartData={chartData}
        dataType={hardwareType}
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
