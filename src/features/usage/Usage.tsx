import {
  cpuUsageHistoryAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/atom/chart";
import { LineChartComponent as LineChart } from "@/components/charts/LineChart";
import { chartConfig } from "@/consts/chart";
import { useSettingsAtom } from "@/hooks/useSettingsAtom";
import { useAtom } from "jotai";
import { useMemo } from "react";

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

const ChartTemplate = () => {
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

  return <div className="p-8">{renderedCharts}</div>;
};

export default ChartTemplate;
