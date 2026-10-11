import { useAtomValue } from "jotai";
import { memo, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { LineChartComponent } from "@/components/charts/LineChart";
import { Sparkline } from "@/components/charts/Sparkline";
import { InfoTable } from "@/components/InfoTable";
import { chartConfig } from "@/consts/chart";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import { useLiveSeries } from "@/features/hardware/hooks/useLiveSeries";
import { useProcessInfo } from "@/features/hardware/hooks/useProcessInfo";
import { processorCountAtom } from "@/features/hardware/store/liveUsage";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { cn } from "@/lib/utils";

const cpuChartLabels = Array(chartConfig.historyLengthSec).fill("");

export const CpuUsages = () => {
  return (
    <div className="p-8">
      <CpuUsageChart />
    </div>
  );
};

/** The overall CPU window. Its own component so the 1 Hz series stops here. */
const CpuHistoryChart = () => {
  const cpuUsageHistory = useLiveSeries({ kind: "cpu" });

  return (
    <LineChartComponent
      labels={cpuChartLabels}
      chartData={cpuUsageHistory}
      dataType="cpu"
      size="lg"
      lineGraphMix={false}
    />
  );
};

const CpuUsageChart = memo(() => {
  // How many charts to draw, not what is in them: each core's chart reads its
  // own series, so a sample does not rebuild this list.
  const processorCount = useAtomValue(processorCountAtom);
  const { init, hardwareInfo } = useHardwareInfoAtom();
  const { processes } = useProcessInfo();
  const { t } = useTranslation();

  // biome-ignore lint/correctness/useExhaustiveDependencies: intentional dependency omission
  useEffect(() => {
    init();
  }, []);

  return (
    <div className="flex flex-col gap-2 xl:flex-row">
      <div className="w-full xl:w-2/6">
        <CpuHistoryChart />
        {hardwareInfo.cpu && (
          <InfoTable
            className="mt-4"
            data={{
              [t("shared.name")]: hardwareInfo.cpu.name,
              [t("shared.vendor")]: hardwareInfo.cpu.vendor,
              [t("shared.coreCount")]: hardwareInfo.cpu.coreCount,
              [t("shared.threadCount")]: processorCount,
              [t("shared.defaultClockSpeed")]:
                `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
              [t("shared.processCount")]: processes.length,
            }}
          />
        )}
      </div>

      <div className="mt-5 ml-3 grid grid-cols-1 gap-5 md:grid-cols-2 lg:w-4/6 xl:grid-cols-4">
        {Array.from({ length: processorCount }, (_, number) => number).map(
          (processorNumber) => (
            <ProcessorChart
              key={processorNumber}
              processorNumber={processorNumber}
            />
          ),
        )}
      </div>
    </div>
  );
});

const ProcessorChart = memo(
  ({ processorNumber }: { processorNumber: number }) => {
    const { settings } = useSettingsAtom();
    const { t } = useTranslation();
    const values = useLiveSeries({ kind: "processor", index: processorNumber });

    return (
      <div
        className={cn(
          "h-[160px] max-h-[200px] max-w-[300px]",
          settings.lineGraphBorder &&
            "rounded-xl border-2 border-slate-400 p-2 dark:border-zinc-600",
        )}
      >
        <Sparkline
          values={values}
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
