import { useAtomValue } from "jotai";
import type { CSSProperties } from "react";
import { LineChartComponent as LineChart } from "@/components/charts/LineChart";
import { BurnInShift } from "@/components/shared/BurnInShift";
import { chartConfig } from "@/consts/chart";
import { graphicUsageHistoryAtom } from "@/features/hardware/store/gpu";
import {
  cpuUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/features/hardware/store/liveUsage";
import { useSettingsAtom } from "@/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";

const labels = Array(chartConfig.historyLengthSec).fill("");

type UsageChartProps = {
  fitToContainer: boolean;
};

type UsageGraphPanelProps = {
  fitToContainer?: boolean;
  height?: string;
  padding?: number;
  className?: string;
  testId?: string;
};

const CpuUsageChart = ({ fitToContainer }: UsageChartProps) => {
  const cpuUsageHistory = useAtomValue(cpuUsageHistoryAtom);
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
  const memoryUsageHistory = useAtomValue(memoryUsageHistoryAtom);
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
  const graphicUsageHistory = useAtomValue(graphicUsageHistoryAtom);
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
  const cpuUsageHistory = useAtomValue(cpuUsageHistoryAtom);
  const memoryUsageHistory = useAtomValue(memoryUsageHistoryAtom);
  const graphicUsageHistory = useAtomValue(graphicUsageHistoryAtom);

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

export const UsageGraphPanel = ({
  fitToContainer: fitToContainerOverride,
  height,
  padding,
  className,
  testId = "usage-graph-panel",
}: UsageGraphPanelProps) => {
  const { settings } = useSettingsAtom();
  const fitToContainer = fitToContainerOverride ?? settings.graphFitToWindow;

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
        height,
        padding: `${padding ?? settings.graphMarginPx}px`,
      } satisfies CSSProperties)
    : undefined;

  return (
    <div
      className={cn(
        fitToContainer
          ? "flex min-h-0 flex-col gap-4 overflow-y-auto"
          : undefined,
        className,
      )}
      style={fitStyle}
      data-testid={testId}
    >
      {renderedCharts}
    </div>
  );
};

export const ChartTemplate = ({
  isFullScreen = false,
}: {
  isFullScreen?: boolean;
}) => {
  const { settings } = useSettingsAtom();
  const fitToContainer = settings.graphFitToWindow;
  const fitHeight =
    "calc(100dvh - var(--burnin-padding) - var(--burnin-padding))";

  return (
    <BurnInShift enabled paddingOverride={fitToContainer ? 0 : undefined}>
      <UsageGraphPanel
        fitToContainer={fitToContainer}
        height={fitHeight}
        className={cn(!fitToContainer && "p-8", !isFullScreen && "ml-16")}
        testId="usage-chart-layout"
      />
    </BurnInShift>
  );
};
