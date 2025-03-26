import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import type { archivePeriods } from "@/consts";
import type {
  ChartDataType,
  DataStats,
  HardwareDataType,
} from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { HardwareType } from "@/rspc/bindings";
import { useTranslation } from "react-i18next";
import { useInsightChart } from "../hooks/useInsightChart";

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
  const { t } = useTranslation();
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
        dataKey={`${t("shared.usage")} (%)`}
      />
    </div>
  );
};

export const GpuInsightChart = ({
  dataType,
  period,
  dataStats,
  offset,
  gpuName,
}: {
  dataType: Exclude<HardwareDataType, "clock">;
  period: (typeof archivePeriods)[number];
  dataStats: DataStats;
  offset: number;
  gpuName: string;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { labels, chartData } = useInsightChart({
    hardwareType: "gpu",
    dataStats,
    dataType,
    period,
    offset,
    gpuName,
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
        dataType={"gpu"}
        chartConfig={chartConfig}
        border={false}
        size="lg"
        lineGraphMix={false}
        lineGraphShowScale={true}
        lineGraphShowTooltip={true}
        lineGraphType={settings.lineGraphType}
        lineGraphShowLegend={false}
        dataKey={
          {
            usage: `GPU ${t("shared.usage")} (%)`,
            temp: `GPU ${t("shared.temperature")} (${settings.temperatureUnit === "C" ? "°C" : "°F"})`,
          }[dataType]
        }
        range={
          dataType === "temp" && settings.temperatureUnit === "F"
            ? [0, 220]
            : [0, 100]
        }
      />
    </div>
  );
};
