import { useTranslation } from "react-i18next";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import type { ChartConfig } from "@/components/ui/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { ChartDataType } from "../../types/hardwareDataType";
import { SnapshotIcon } from "../icons/snapshot";
import { ProcessHistoryTable } from "./components/ProcessHistoryTable";
import { SnapshotChart } from "./components/SnapshotChart";
import { SnapshotControls } from "./components/SnapshotControls";
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
    filteredProcessData,
    totalMemoryMB,
    memoryMaxOption,
    setMemoryMaxOption,
    selectedMemoryMaxMB,
  } = useSnapshot();
  const { settings } = useSettingsAtom();
  const { t } = useTranslation();

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
    <div className="my-4 space-y-6">
      {/* Controls Card */}
      <Card>
        <CardHeader className="pb-4">
          <CardTitle className="flex items-center gap-2">
            <SnapshotIcon size={24} color="currentColor" />
            {t("shared.snapshot.title")}
          </CardTitle>
          <CardDescription>{t("shared.snapshot.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            <SnapshotControls
              period={period}
              setPeriod={setPeriod}
              cpuRange={cpuRange}
              setCpuRange={setCpuRange}
              memoryRange={memoryRange}
              setMemoryRange={setMemoryRange}
              selectedDataType={selectedDataType}
              setSelectedDataType={setSelectedDataType}
              selectedMemoryMaxMB={selectedMemoryMaxMB}
              memoryMaxOption={memoryMaxOption}
              setMemoryMaxOption={setMemoryMaxOption}
              totalMemoryMB={totalMemoryMB}
            />
          </div>
        </CardContent>
      </Card>

      {/* Chart Card */}
      <Card>
        <CardContent className="pt-6">
          <SnapshotChart
            labels={filledLabels}
            chartData={filledChartData}
            selectedDataType={selectedDataType}
            chartConfig={chartConfig}
            settings={settings}
          />
        </CardContent>
      </Card>

      {/* Process Results Card */}
      {filteredProcessData.length === 0 ? (
        <div className="space-y-2 py-8 text-center">
          <p className="text-muted-foreground">
            {t("shared.snapshot.noDataMessage")}
          </p>
          <p className="text-muted-foreground text-sm">
            {t("shared.snapshot.adjustFiltersHint")}
          </p>
        </div>
      ) : (
        <ProcessHistoryTable
          processStats={filteredProcessData}
          loading={false}
        />
      )}
    </div>
  );
};
