import { memo } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";
import { Sparkline } from "@/components/charts/Sparkline";
import { displayHardType } from "@/features/hardware/consts/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useWindowSize } from "@/hooks/useWindowSize";
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
    usage: (number | null)[];
  }) => {
    const { settings } = useSettingsAtom();
    const { t } = useTranslation();
    const { isBreak } = useWindowSize();

    return (
      <div
        className={miniLineChartVariant({ isBackground: !isBreak("lg") })}
        style={{ height: isBreak("xl") ? 160 : 100 }}
      >
        <Sparkline
          values={usage}
          colorRgb={settings.lineGraphColor[hardwareType]}
          lineGraphType={settings.lineGraphType}
          fill={settings.lineGraphFill}
          showScale={false}
          tooltip={{
            label: displayHardType[hardwareType],
            format: (value) => `${value}% ${t("shared.usage").toLowerCase()}`,
          }}
        />
      </div>
    );
  },
);
