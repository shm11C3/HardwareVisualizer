import { ChartLineUpIcon, LightningIcon } from "@phosphor-icons/react";
import { useAtomValue } from "jotai";
import { useTranslation } from "react-i18next";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { powerDrawAvailableAtom } from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import {
  type PerformancePowerMode,
  performancePowerModes,
} from "../types/performanceLayout";

const modeIcons = {
  current: <LightningIcon />,
  graph: <ChartLineUpIcon />,
} satisfies Record<PerformancePowerMode, React.ReactNode>;

export const PowerDisplayModeSwitcher = ({
  mode,
  onModeChange,
  compact = false,
  className,
}: {
  mode: PerformancePowerMode;
  onModeChange: (mode: PerformancePowerMode) => void;
  compact?: boolean;
  className?: string;
}) => {
  const { t } = useTranslation();
  const powerAvailable = useAtomValue(powerDrawAvailableAtom);
  const { settings } = useSettingsAtom();

  if (!powerAvailable || settings.powerDisplayTargets.length === 0) {
    return null;
  }

  return (
    <Tabs
      value={mode}
      onValueChange={(value) => onModeChange(value as PerformancePowerMode)}
      className={cn("min-w-0", className)}
    >
      <TabsList
        className={cn(
          "h-auto max-w-full justify-start overflow-x-auto",
          compact && "gap-0.5 p-0.5",
        )}
        aria-label={t("pages.performance.powerModeSwitcher")}
        data-testid="performance-power-mode-switcher"
      >
        {performancePowerModes.map((candidate) => (
          <TabsTrigger
            key={candidate}
            value={candidate}
            className={cn("min-h-9 px-3", compact && "min-h-8 px-2 text-xs")}
          >
            {modeIcons[candidate]}
            {t(`pages.performance.powerModes.${candidate}`)}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
};
