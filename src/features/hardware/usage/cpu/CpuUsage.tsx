import {
  LineChartComponent,
  SingleLineChart,
} from "@/components/charts/LineChart";
import { InfoTable } from "@/components/shared/InfoTable";
import type { ChartConfig } from "@/components/ui/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { transpose } from "@/lib/array";
import { useAtom } from "jotai";
import { memo, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { chartConfig } from "../../consts/chart";
import { useHardwareInfoAtom } from "../../hooks/useHardwareInfoAtom";
import { useProcessInfo } from "../../hooks/useProcessInfo";
import {
  cpuUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "../../store/chart";

export const CpuUsages = () => {
  return (
    <div className="p-8">
      <CpuUsageChart />
    </div>
  );
};

const CpuUsageChart = memo(() => {
  const [processorsUsageHistory] = useAtom(processorsUsageHistoryAtom);
  const [cpuUsageHistory] = useAtom(cpuUsageHistoryAtom);
  const { init, hardwareInfo } = useHardwareInfoAtom();
  const processes = useProcessInfo();
  const { t } = useTranslation();

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
  useEffect(() => {
    init();
  }, []);

  return (
    <div className="flex gap-2">
      <div className="w-2/6">
        <LineChartComponent
          labels={Array(chartConfig.historyLengthSec).fill("")}
          chartData={cpuUsageHistory}
          dataType="cpu"
          size="lg"
          lineGraphMix={false}
        />
        {hardwareInfo.cpu && (
          <InfoTable
            className="mt-4"
            data={{
              [t("shared.name")]: hardwareInfo.cpu.name,
              [t("shared.vendor")]: hardwareInfo.cpu.vendor,
              [t("shared.coreCount")]: hardwareInfo.cpu.coreCount,
              [t("shared.threadCount")]: processorsUsageHistory[0]?.length || 0,
              [t("shared.defaultClockSpeed")]:
                `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
              [t("shared.processCount")]: processes.length,
            }}
          />
        )}
      </div>

      <div className="mt-5 grid w-4/6 grid-cols-4 gap-2">
        {transpose(processorsUsageHistory)
          .map((processorData, index) => {
            return { data: processorData, id: index };
          })
          .map((processorData) => (
            <ProcessorChart
              key={processorData.id}
              data={processorData.data}
              processorNumber={processorData.id}
            />
          ))}
      </div>
    </div>
  );
});

const ProcessorChart = memo(
  ({ data, processorNumber }: { data: number[]; processorNumber: number }) => {
    const { settings } = useSettingsAtom();
    const { t } = useTranslation();

    const config = {
      cpu: {
        label: `Processor-${processorNumber}`,
        color: settings.lineGraphColor.cpu,
      },
    } satisfies ChartConfig;

    return (
      <SingleLineChart
        labels={Array(chartConfig.historyLengthSec).fill("")}
        chartData={Array(
          Math.max(chartConfig.historyLengthSec - data.length, 0),
        )
          .fill(null)
          .concat(data)}
        dataType="cpu"
        size="sm"
        lineGraphMix={false}
        chartConfig={config}
        border={settings.lineGraphBorder}
        lineGraphShowScale={settings.lineGraphShowScale}
        lineGraphShowTooltip={settings.lineGraphShowTooltip}
        lineGraphType={settings.lineGraphType}
        lineGraphShowLegend={false}
        dataKey={`${t("shared.usage")} (%)`}
        width={300}
        height={200}
      />
    );
  },
);
