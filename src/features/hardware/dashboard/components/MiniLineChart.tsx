import { memo } from "react";
import { useTranslation } from "react-i18next";
import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import { chartConfig as charConst } from "@/features/hardware/consts/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { ChartDataType } from "../../types/hardwareDataType";

export const MiniLineChart = memo(
  ({
    hardwareType,
    usage,
  }: {
    hardwareType: ChartDataType;
    usage: number[];
  }) => {
    const { settings } = useSettingsAtom();
    const { t } = useTranslation();

    const chartConfig: Record<ChartDataType, { label: string; color: string }> =
      {
        cpu: {
          label: "CPU",
          color: settings.lineGraphColor.cpu,
        },
        memory: {
          label: "RAM",
          color: settings.lineGraphColor.memory,
        },
        gpu: {
          label: "GPU",
          color: settings.lineGraphColor.gpu,
        },
      } satisfies ChartConfig;

    const labels = Array(charConst.historyLengthSec).fill("");

    return (
      <div className="w-[300px]">
        <SingleLineChart
          labels={labels}
          chartData={usage}
          dataType={hardwareType}
          chartConfig={chartConfig}
          border={false}
          size="sm"
          lineGraphMix={false}
          lineGraphShowScale={false}
          lineGraphShowTooltip={true}
          lineGraphType={settings.lineGraphType}
          lineGraphShowLegend={false}
          dataKey={`${t("shared.usage")} (%)`}
          height={160}
          width={300}
        />
      </div>
    );
  },
);
