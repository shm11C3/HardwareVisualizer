import { memo } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";
import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import { chartConfig as charConst } from "@/features/hardware/consts/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useWindowsSize } from "@/hooks/useWindowSize";
import type { ChartDataType } from "../../types/hardwareDataType";

const miniLineChartVariant = tv({
  base: "xl:w-[300px]",
  variants: {
    isBackground: {
      true: "w-5/6 top-40 absolute opacity-50",
      false: "w-[200px]",
    },
  },
});

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
    const { isBreak } = useWindowsSize();

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
      <div className={miniLineChartVariant({ isBackground: !isBreak("lg") })}>
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
          height={isBreak("xl") ? 160 : 100}
          width="stretch"
        />
      </div>
    );
  },
);
