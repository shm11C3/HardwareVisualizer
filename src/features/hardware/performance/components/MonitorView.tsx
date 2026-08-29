import { useAtomValue } from "jotai";
import type { CSSProperties } from "react";
import { powerDrawAvailableAtom } from "@/features/hardware/store/chart";
import { UsageGraphPanel } from "@/features/hardware/usage/Usage";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { PerformanceMonitorPowerMode } from "../types/performanceLayout";
import { PowerDrawChart } from "./PowerDrawChart";
import { PowerDrawRail } from "./PowerDrawRail";

export const MonitorView = ({
  powerMode,
}: {
  powerMode: PerformanceMonitorPowerMode;
}) => {
  const powerAvailable = useAtomValue(powerDrawAvailableAtom);
  const { settings } = useSettingsAtom();
  const showPower = powerAvailable && settings.powerDisplayTargets.length > 0;

  return (
    <div
      className="flex min-h-0 flex-1 flex-col gap-4"
      style={
        {
          padding: `${settings.graphMarginPx}px`,
        } satisfies CSSProperties
      }
      data-power-mode={powerMode}
    >
      {!showPower ? (
        <UsageGraphPanel
          fitToContainer
          padding={0}
          className="min-h-0 flex-1"
          testId="performance-usage-graphs"
        />
      ) : (
        <>
          {powerMode === "current" ? <PowerDrawRail /> : null}
          <UsageGraphPanel
            fitToContainer
            padding={0}
            className="min-h-0 flex-[3]"
            testId="performance-usage-graphs"
          />
          {powerMode === "graph" ? <PowerDrawChart /> : null}
        </>
      )}
    </div>
  );
};
