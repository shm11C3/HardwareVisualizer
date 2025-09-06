import { useAtom } from "jotai";
import { useMemo } from "react";
import { LineChartComponent as LineChart } from "@/components/charts/LineChart";
import { BurnInShift } from "@/components/shared/BurnInShift";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  cpuUsageHistoryAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

const labels = Array(chartConfig.historyLengthSec).fill("");

const CpuUsageChart = () => {
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);
  const { settings } = useSettingsAtom();

  return (
    <LineChart
      labels={labels}
      chartData={cpuUsageHistory}
      dataType="cpu"
      size={settings.graphSize}
      lineGraphMix={false}
    />
  );
};

const MemoryUsageChart = () => {
  const [memoryUsageHistory] = useAtom(memoryUsageHistoryAtom);
  const { settings } = useSettingsAtom();

  return (
    <LineChart
      labels={labels}
      chartData={memoryUsageHistory}
      dataType="memory"
      size={settings.graphSize}
      lineGraphMix={false}
    />
  );
};

const GpuUsageChart = () => {
  const [graphicUsageHistory] = useAtom(graphicUsageHistoryAtom);
  const { settings } = useSettingsAtom();

  return (
    <LineChart
      labels={labels}
      chartData={graphicUsageHistory}
      dataType="gpu"
      size={settings.graphSize}
      lineGraphMix={false}
    />
  );
};

const MixUsageChart = () => {
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
    />
  );
};

export const ChartTemplate = () => {
  const { settings } = useSettingsAtom();

  const renderedCharts = useMemo(() => {
    return settings.lineGraphMix ? (
      <MixUsageChart />
    ) : (
      <>
        {settings.displayTargets.includes("cpu") && <CpuUsageChart />}
        {settings.displayTargets.includes("memory") && <MemoryUsageChart />}
        {settings.displayTargets.includes("gpu") && <GpuUsageChart />}
      </>
    );
  }, [settings]);

  return (
    <BurnInShift enabled>
      <div className="ml-16 p-8">{renderedCharts}</div>
    </BurnInShift>
  );
};
