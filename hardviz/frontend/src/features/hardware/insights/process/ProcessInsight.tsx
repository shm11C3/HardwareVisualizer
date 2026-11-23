import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { LoadFailure } from "@/components/LoadFailure";
import { processStatsPeriods } from "@/features/hardware/consts/chart";
import { SelectPeriod } from "@/features/hardware/insights/components/SelectPeriod";
import { useArchivePeriodOptions } from "@/features/hardware/insights/hooks/useArchivePeriodOptions";
import type { ProcessStatsPeriod } from "@/features/hardware/insights/utils/archivePeriod";
import { useTauriStore } from "@/hooks/tauri/useTauriStore";
import { ProcessBubbleChart } from "./chart/Bubble";
import { ProcessGpuNotice } from "./components/ProcessGpuNotice";
import { gpuReadForWindow, joinProcessGpuUsage } from "./funcs/processGpuUsage";
import { useProcessGpuStats } from "./hooks/useProcessGpuStats";
import { useProcessStats } from "./hooks/useProcessStats";
import { ProcessTable } from "./table/ProcessTable";

export const ProcessInsight = () => {
  const [period, setPeriod] = useTauriStore<ProcessStatsPeriod>(
    "periodProcessStats",
    60,
  );

  const options = useArchivePeriodOptions(processStatsPeriods);

  return (
    <div className="pb-6">
      <div className="sticky top-[8px] z-40 mr-1 flex items-center justify-end">
        <SelectPeriod
          options={options}
          selected={period}
          onChange={setPeriod}
        />
      </div>
      {period != null && <ProcessArea period={period} />}
    </div>
  );
};

const ProcessArea = ({ period }: { period: ProcessStatsPeriod }) => {
  const { t } = useTranslation();
  const { processStats, loading, hasError, retry, endAt, statsWindow } =
    useProcessStats({
      period,
      offset: 0,
    });
  const gpu = useProcessGpuStats({ period, endAt });
  // Only a GPU read for the shown stats' window joins the table. While it is
  // pending the column stays hidden: an older read's values would belong to
  // another period, and its "—" would wrongly claim "no record here".
  const gpuRead = gpuReadForWindow(gpu.read, statsWindow);
  const gpuRecords =
    gpuRead?.status === "ready" && gpuRead.stats.kind === "available"
      ? gpuRead.stats.records
      : null;

  const filteredData = useMemo(() => {
    if (processStats == null) {
      return [];
    }

    return processStats.filter(
      (stat) =>
        stat.total_execution_sec > 0 &&
        stat.total_execution_sec < 60 * 60 * 24 * 30, // Ignore processes running for more than 1 month
    );
  }, [processStats]);

  const tableRows = useMemo(
    () => joinProcessGpuUsage(filteredData, gpuRecords ?? []),
    [filteredData, gpuRecords],
  );

  if (hasError) {
    return (
      <LoadFailure
        className="mt-4 py-16"
        message={t("shared.loadFailed")}
        onRetry={retry}
      />
    );
  }

  return (
    <div className="mt-4">
      <ProcessBubbleChart processStats={filteredData} loading={loading} />
      <ProcessGpuNotice read={gpuRead} onRetry={gpu.retry} />
      <ProcessTable
        processStats={tableRows}
        loading={loading}
        period={period}
        showGpuUsage={gpuRecords != null}
      />
    </div>
  );
};
