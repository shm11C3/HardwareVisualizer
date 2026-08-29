import { ChartLineUpIcon, LightningIcon } from "@phosphor-icons/react";
import { useAtomValue } from "jotai";
import { useTranslation } from "react-i18next";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { powerDrawAvailableAtom } from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import {
  type PerformanceMonitorPowerMode,
  performanceMonitorPowerModes,
} from "../types/performanceLayout";

const modeIcons = {
  current: <LightningIcon />,
  graph: <ChartLineUpIcon />,
} satisfies Record<PerformanceMonitorPowerMode, React.ReactNode>;

export const MonitorPowerModeSwitcher = ({
  mode,
  onModeChange,
}: {
  mode: PerformanceMonitorPowerMode;
  onModeChange: (mode: PerformanceMonitorPowerMode) => void;
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
      onValueChange={(value) =>
        onModeChange(value as PerformanceMonitorPowerMode)
      }
      className="min-w-0"
    >
      <TabsList
        className="h-auto max-w-full justify-start overflow-x-auto"
        aria-label={t("pages.performance.monitorPowerModeSwitcher")}
        data-testid="performance-monitor-power-mode-switcher"
      >
        {performanceMonitorPowerModes.map((candidate) => (
          <TabsTrigger
            key={candidate}
            value={candidate}
            className="min-h-9 px-3"
          >
            {modeIcons[candidate]}
            {t(`pages.performance.monitorPowerModes.${candidate}`)}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
};
