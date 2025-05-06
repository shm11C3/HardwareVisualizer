import { useTauriStore } from "@/hooks/useTauriStore";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { archivePeriods } from "../../consts/chart";
import { SelectPeriod } from "../components/SelectPeriod";
import { ProcessBubbleChart } from "./chart/Bubble";
import { useProcessStats } from "./hooks/useProcessStats";
import { ProcessTable } from "./table/ProcessTable";

export const ProcessInsight = () => {
  const { t } = useTranslation();
  const [period, setPeriod] = useTauriStore<(typeof archivePeriods)[number]>(
    "periodProcessStats",
    60,
  );

  const periods: Record<(typeof archivePeriods)[number], string> = {
    "10": `10 ${t("shared.time.minutes")}`,
    "30": `30 ${t("shared.time.minutes")}`,
    "60": `1 ${t("shared.time.hours")}`,
    "180": `3 ${t("shared.time.hours")}`,
    "720": `12 ${t("shared.time.hours")}`,
    "1440": `1 ${t("shared.time.days")}`,
    "10080": `7 ${t("shared.time.days")}`,
    "20160": `14 ${t("shared.time.days")}`,
    "43200": `30 ${t("shared.time.days")}`,
  };

  const options = archivePeriods.map((period) => ({
    label: periods[period],
    value: period,
  }));

  return (
    <div className="pb-6">
      <div className="sticky top-[8px] z-50 mr-1 flex items-center justify-end">
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

const ProcessArea = ({
  period,
}: { period: (typeof archivePeriods)[number] }) => {
  const { processStats, loading } = useProcessStats({
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
        stat.total_execution_sec < 60 * 60 * 24 * 30, // 1ヶ月以上稼働しているものは無視する
    );
  }, [processStats]);

  return (
    <div className="mt-4">
      <ProcessBubbleChart processStats={filteredData} loading={loading} />
      <ProcessTable processStats={filteredData} loading={loading} />
    </div>
  );
};
