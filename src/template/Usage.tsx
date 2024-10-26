import {
  cpuUsageHistoryAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/atom/chart";
import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { LineChart } from "@/components/charts/LineChart";
import { chartConfig } from "@/consts/chart";
import { useAtom } from "jotai";
import { useMemo } from "react";

const labels = Array(chartConfig.historyLengthSec).fill("");

const CpuUsageChart = () => {
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);

  return (
    <LineChart
      labels={labels}
      chartData={cpuUsageHistory}
      dataType="cpu"
      lineGraphMix={false}
    />
  );
};

const MemoryUsageChart = () => {
  const [memoryUsageHistory] = useAtom(memoryUsageHistoryAtom);

  return (
    <LineChart
      labels={labels}
      chartData={memoryUsageHistory}
      dataType="memory"
      lineGraphMix={false}
    />
  );
};

const GpuUsageChart = () => {
  const [graphicUsageHistory] = useAtom(graphicUsageHistoryAtom);

  return (
    <LineChart
      labels={labels}
      chartData={graphicUsageHistory}
      dataType="gpu"
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
