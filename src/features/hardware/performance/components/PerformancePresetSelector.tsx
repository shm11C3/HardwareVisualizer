import {
  ArrowsInSimpleIcon,
  ChartLineUpIcon,
  SlidersHorizontalIcon,
  SquaresFourIcon,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  type PerformanceLayoutPreset,
  performanceLayoutPresets,
} from "../types/performanceLayout";

const presetIcons = {
  compact: <ArrowsInSimpleIcon />,
  monitor: <ChartLineUpIcon />,
  detailed: <SquaresFourIcon />,
  custom: <SlidersHorizontalIcon />,
} satisfies Record<PerformanceLayoutPreset, React.ReactNode>;

export const PerformancePresetSelector = ({
  preset,
  onPresetChange,
}: {
  preset: PerformanceLayoutPreset;
  onPresetChange: (preset: PerformanceLayoutPreset) => void;
}) => {
  const { t } = useTranslation();

  return (
    <Tabs
      value={preset}
      onValueChange={(value) =>
        onPresetChange(value as PerformanceLayoutPreset)
      }
      className="min-w-0"
    >
      <TabsList
        className="h-auto max-w-full justify-start overflow-x-auto"
        aria-label={t("pages.performance.layoutPreset")}
      >
        {performanceLayoutPresets.map((candidate) => (
          <TabsTrigger
            key={candidate}
            value={candidate}
            className="min-h-9 px-3"
          >
            {presetIcons[candidate]}
            {t(`pages.performance.presets.${candidate}.name`)}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
};
