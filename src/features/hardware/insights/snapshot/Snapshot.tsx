import { CpuIcon, MemoryIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { SingleLineChart } from "@/components/charts/LineChart";
import type { ChartConfig } from "@/components/ui/chart";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { ChartDataType } from "../../types/hardwareDataType";
import { ProcessHistoryTable } from "./components/ProcessHistoryTable";
import {
  SelectMemoryMaxOption,
  SelectPeriod,
  SelectRange,
} from "./components/SnapshotForm";
import { useSnapshot } from "./hooks/useSnapshot";

export const Snapshot = () => {
  const {
    period,
    setPeriod,
    cpuRange,
    setCpuRange,
    memoryRange,
    setMemoryRange,
    selectedDataType,
    setSelectedDataType,
    filledLabels,
    filledChartData,
    processData,
    filteredProcessData,
    totalMemoryMB,
    memoryMaxOption,
    setMemoryMaxOption,
    selectedMemoryMaxMB,
  } = useSnapshot();
  const { settings } = useSettingsAtom();
  const { t } = useTranslation();

  // メモリ値のフォーマット関数
  const formatMemoryValue = (mb: number) => {
    if (mb >= 1024) {
      return `${Math.round(mb / 1024 * 100) / 100}GB`;
    }
    return `${mb}MB`;
  };

  const chartConfig: Record<
    Exclude<ChartDataType, "gpu">,
    { label: string; color: string }
  > = {
    cpu: {
      label: "CPU",
      color: settings.lineGraphColor.cpu,
    },
    memory: {
      label: "RAM",
      color: settings.lineGraphColor.memory,
    },
  } satisfies ChartConfig;

  return (
    <div>
      <div className="mt-2 flex items-center justify-between gap-5">
        <div className="flex w-full gap-4">
          <div className="flex-1">
            <SelectRange
              label={`Process CPU Usage Range (${cpuRange.value[0]}% - ${cpuRange.value[1]}%)`}
              range={cpuRange}
              setRange={setCpuRange}
            />
          </div>
          <div className="flex-1">
            <SelectRange
              label={`Process Memory Usage Range (${formatMemoryValue(memoryRange.value[0])} - ${formatMemoryValue(memoryRange.value[1])})`}
              range={memoryRange}
              setRange={setMemoryRange}
              max={selectedMemoryMaxMB}
            />
          </div>
          <div className="w-48">
            <SelectMemoryMaxOption
              memoryMaxOption={memoryMaxOption}
              setMemoryMaxOption={setMemoryMaxOption}
              totalMemoryMB={totalMemoryMB}
            />
          </div>
        </div>

        <SelectPeriod period={period} setPeriod={setPeriod} />
      </div>

      {/** データタイプの選択 */}
      <RadioGroup
        className="mt-4 flex gap-4"
        defaultValue="cpu"
        onValueChange={(e) => setSelectedDataType(e as "cpu" | "memory")}
      >
        <Label className="flex items-center gap-2">
          <RadioGroupItem value="cpu" className="h-4 w-4 text-blue-600" />
          <span className="flex items-center gap-1">
            <CpuIcon color={`rgb(${settings.lineGraphColor.cpu})`} />
            CPU
          </span>
        </Label>
        <Label className="flex items-center gap-2">
          <RadioGroupItem value="memory" className="h-4 w-4 text-blue-600" />
          <span className="flex items-center gap-1">
            <MemoryIcon color={`rgb(${settings.lineGraphColor.memory})`} />
            RAM
          </span>
        </Label>
      </RadioGroup>

      {/** 選択された範囲のCPU使用率・メモリ使用率 */}
      <SingleLineChart
        className="mt-5"
        labels={filledLabels}
        chartData={filledChartData}
        dataType={selectedDataType}
        chartConfig={chartConfig}
        border={false}
        size="lg"
        lineGraphMix={false}
        lineGraphShowScale={true}
        lineGraphShowTooltip={true}
        lineGraphType={settings.lineGraphType}
        lineGraphShowLegend={false}
        dataKey={`${t("shared.usage")} (%)`}
      />

      {/** 選択された範囲のプロセス */}
      <div className="mt-6">
        <ProcessHistoryTable
          processStats={filteredProcessData}
          loading={false}
        />
      </div>
    </div>
  );
};
