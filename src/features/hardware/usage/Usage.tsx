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

type UsageGraphPanelProps = {
  fitToContainer?: boolean;
  height?: string;
  padding?: number;
  className?: string;
  testId?: string;
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
