import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { LineChartComponent as LineChart } from "@/components/charts/LineChart";
import { chartConfig } from "@/consts/chart";

export const PreviewChart = () => {
  const { settings } = useSettingsAtom();

  const labels = Array(chartConfig.historyLengthSec).fill("");

  const cpuValues = [
    7, 7, 8, 9, 11, 10, 5, 9, 7, 9, 7, 5, 4, 6, 6, 6, 10, 6, 8, 9, 8, 7, 7, 4,
    7, 6, 7, 6, 6, 6, 8, 10, 8, 5, 5, 6, 6, 7, 24, 20, 19, 24, 18, 7, 8, 17, 11,
    15, 22, 10, 22, 12, 6, 6, 6, 8, 10, 12, 10, 7,
  ];

  const memoryValues = [
    59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59,
    59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59,
    59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 59, 60, 60, 60, 60, 60, 60, 60,
    60, 60, 60,
  ];

  const gpuValues = [
    9, 31, 3, 4, 0, 6, 0, 0, 3, 0, 1, 0, 0, 0, 0, 3, 8, 2, 2, 0, 8, 1, 0, 0, 0,
    4, 1, 0, 1, 3, 7, 0, 2, 0, 0, 4, 2, 6, 23, 25, 31, 22, 25, 27, 3, 12, 2, 30,
    17, 4, 1, 2, 0, 0, 1, 3, 3, 8, 4, 1,
  ];

  return settings.lineGraphMix ? (
    <LineChart
      labels={labels}
      cpuData={settings.displayTargets.includes("cpu") ? cpuValues : []}
      memoryData={
        settings.displayTargets.includes("memory") ? memoryValues : []
      }
      gpuData={settings.displayTargets.includes("gpu") ? gpuValues : []}
      size="lg"
      lineGraphMix={true}
    />
  ) : (
    <>
      <LineChart
        labels={labels}
        chartData={cpuValues}
        dataType="cpu"
        size="lg"
        lineGraphMix={false}
      />
      <LineChart
        labels={labels}
        chartData={memoryValues}
        dataType="memory"
        size="lg"
        lineGraphMix={false}
      />
      <LineChart
        labels={labels}
        chartData={gpuValues}
        dataType="gpu"
        size="lg"
        lineGraphMix={false}
      />
    </>
  );
};
