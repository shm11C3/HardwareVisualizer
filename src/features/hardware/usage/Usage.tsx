import { useAtom } from "jotai";
import type { CSSProperties } from "react";
import { LineChartComponent as LineChart } from "@/components/charts/LineChart";
import { BurnInShift } from "@/components/shared/BurnInShift";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  cpuUsageHistoryAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";

const labels = Array(chartConfig.historyLengthSec).fill("");

type UsageChartProps = {
  fitToContainer: boolean;
};

const CpuUsageChart = ({ fitToContainer }: UsageChartProps) => {
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);
  const { settings } = useSettingsAtom();

  return (
    <LineChart
      labels={labels}
      chartData={cpuUsageHistory}
      dataType="cpu"
      size={settings.graphSize}
      lineGraphMix={false}
      fitToContainer={fitToContainer}
    />
  );
};

const MemoryUsageChart = ({ fitToContainer }: UsageChartProps) => {
  const [memoryUsageHistory] = useAtom(memoryUsageHistoryAtom);
  const { settings } = useSettingsAtom();

  return (
    <LineChart
      labels={labels}
      chartData={memoryUsageHistory}
      dataType="memory"
      size={settings.graphSize}
      lineGraphMix={false}
      fitToContainer={fitToContainer}
    />
  );
};

const GpuUsageChart = ({ fitToContainer }: UsageChartProps) => {
  const [graphicUsageHistory] = useAtom(graphicUsageHistoryAtom);
  const { settings } = useSettingsAtom();

  return (
    <LineChart
      labels={labels}
      chartData={graphicUsageHistory}
      dataType="gpu"
      size={settings.graphSize}
      lineGraphMix={false}
      fitToContainer={fitToContainer}
    />
  );
};

const MixUsageChart = ({ fitToContainer }: UsageChartProps) => {
  const { settings } = useSettingsAtom();
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);
  const [memoryUsageHistory] = useAtom(memoryUsageHistoryAtom);
  const [graphicUsageHistory] = useAtom(graphicUsageHistoryAtom);

  return (
    <LineChart
      labels={labels}
      cpuData={settings.displayTargets.includes("cpu") ? cpuUsageHistory : []}
      memoryData={
        settings.displayTargets.includes("memory") ? memoryUsageHistory : []
      }
      gpuData={
        settings.displayTargets.includes("gpu") ? graphicUsageHistory : []
      }
      size={settings.graphSize}
      lineGraphMix={true}
      fitToContainer={fitToContainer}
    />
  );
};

export const ChartTemplate = ({
  isFullScreen = false,
}: {
  isFullScreen?: boolean;
}) => {
  const { settings } = useSettingsAtom();
  const fitToContainer = settings.graphFitToWindow;

  const renderedCharts = settings.lineGraphMix ? (
    <MixUsageChart fitToContainer={fitToContainer} />
  ) : (
    <>
      {settings.displayTargets.includes("cpu") && (
        <CpuUsageChart fitToContainer={fitToContainer} />
      )}
      {settings.displayTargets.includes("memory") && (
        <MemoryUsageChart fitToContainer={fitToContainer} />
      )}
      {settings.displayTargets.includes("gpu") && (
        <GpuUsageChart fitToContainer={fitToContainer} />
      )}
    </>
  );

  const fitStyle = fitToContainer
    ? ({
        height: "calc(100dvh - var(--burnin-padding) - var(--burnin-padding))",
        padding: `${settings.graphMarginPx}px`,
      } satisfies CSSProperties)
    : undefined;

  return (
    <BurnInShift enabled paddingOverride={fitToContainer ? 0 : undefined}>
      <div
        className={cn(
          fitToContainer
            ? "flex min-h-0 flex-col gap-4 overflow-y-auto"
            : "p-8",
          !isFullScreen && "ml-16",
        )}
        style={fitStyle}
        data-testid="usage-chart-layout"
      >
        {renderedCharts}
      </div>
    </BurnInShift>
  );
};
