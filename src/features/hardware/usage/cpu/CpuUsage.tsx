import { useAtom } from "jotai";
import { memo, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { LineChartComponent } from "@/components/charts/LineChart";
import { Sparkline } from "@/components/charts/Sparkline";
import { InfoTable } from "@/components/shared/InfoTable";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { transpose } from "@/lib/array";
import { cn } from "@/lib/utils";
import { chartConfig } from "../../consts/chart";
import { useHardwareInfoAtom } from "../../hooks/useHardwareInfoAtom";
import { useProcessInfo } from "../../hooks/useProcessInfo";
import {
  cpuUsageHistoryAtom,
  processorsUsageHistoryAtom,
} from "../../store/chart";

/** Right-align a partially filled per-core history against the chart window. */
const padHistory = (data: number[]): (number | null)[] =>
  Array<number | null>(Math.max(chartConfig.historyLengthSec - data.length, 0))
    .fill(null)
    .concat(data)
    .slice(-chartConfig.historyLengthSec);

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

  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional dependency omission
  useEffect(() => {
    init();
  }, []);

  return (
    <div className="flex flex-col gap-2 xl:flex-row">
      <div className="w-full xl:w-2/6">
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

      <div className="mt-5 ml-3 grid grid-cols-1 gap-5 md:grid-cols-2 lg:w-4/6 xl:grid-cols-4">
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

    return (
      <div
        className={cn(
          "h-[160px] max-h-[200px] max-w-[300px]",
          settings.lineGraphBorder &&
            "rounded-xl border-2 border-slate-400 p-2 dark:border-zinc-600",
        )}
      >
        <Sparkline
          values={padHistory(data)}
          colorRgb={settings.lineGraphColor.cpu}
          lineGraphType={settings.lineGraphType}
          fill={settings.lineGraphFill}
          showScale={settings.lineGraphShowScale}
          {...(settings.lineGraphShowTooltip && {
            tooltip: {
              label: `Processor-${processorNumber}`,
              format: (value: number) =>
                `${value}% ${t("shared.usage").toLowerCase()}`,
            },
          })}
        />
      </div>
    );
  },
);
