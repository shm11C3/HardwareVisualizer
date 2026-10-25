import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { LoadFailure } from "@/components/LoadFailure";
import { processStatsPeriods } from "@/features/hardware/consts/chart";
import { SelectPeriod } from "@/features/hardware/insights/components/SelectPeriod";
import { useArchivePeriodOptions } from "@/features/hardware/insights/hooks/useArchivePeriodOptions";
import type { ProcessStatsPeriod } from "@/features/hardware/insights/utils/archivePeriod";
import { useTauriStore } from "@/hooks/tauri/useTauriStore";
import { ProcessBubbleChart } from "./chart/Bubble";
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
  const { processStats, loading, hasError, retry } = useProcessStats({
    period,
    offset: 0,
  });

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
      <ProcessTable processStats={filteredData} loading={loading} />
    </div>
  );
};
