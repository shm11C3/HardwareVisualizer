import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import type { archivePeriods } from "@/features/hardware/consts/chart";
import type {
  ChartDataType,
  DataStats,
  GpuDataType,
} from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { HardwareType } from "@/rspc/bindings";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useHardwareInfoAtom } from "../../hooks/useHardwareInfoAtom";
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
  dataType: GpuDataType;
  period: (typeof archivePeriods)[number];
  dataStats: DataStats;
  offset: number;
  gpuName: string;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { hardwareInfo } = useHardwareInfoAtom();
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

  const maxDedicatedGpuMemory = useMemo(() => {
    const max =
      hardwareInfo.gpus?.reduce((acc, gpu) => {
        const matches = [
          ...gpu.memorySizeDedicated.matchAll(/(\d+(\.\d+)?|\D+)/g),
        ];

        if (matches.length < 2) {
          return acc;
        }

        const value = Number.parseFloat(matches[0][0]);
        const unit = matches[1][0].trim();

        return Math.max(
          acc,
          Number.parseFloat(
            (
              value * (unit === "GB" ? 1 : unit === "MB" ? 0.001 : 0.000001)
            ).toFixed(1),
          ),
        );
      }, 0) ?? 0;

    // 1 GB未満は1GBとして扱う
    if (max < 1) {
      return 1;
    }

    return max;
  }, [hardwareInfo.gpus]);

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
            dedicatedMemory: `${t("shared.memorySizeDedicatedUsage")} (GB)`,
          }[dataType]
        }
        range={(() => {
          switch (dataType) {
            case "dedicatedMemory":
              return [0, maxDedicatedGpuMemory];
            case "temp":
              return settings.temperatureUnit === "C" ? [0, 100] : [0, 220];
            default:
              return [0, 100];
          }
        })()}
      />
    </div>
  );
};
