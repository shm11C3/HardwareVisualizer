import { useTauriStore } from "@/hooks/useTauriStore";
import { useTranslation } from "react-i18next";
import { archivePeriods } from "../../consts/chart";
import { SelectPeriod } from "../components/SelectPeriod";
import ProcessTable from "./table/ProcessTable";

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

  if (period == null) {
    return <></>;
  }

  return (
    <div className="pb-6">
      <div className="flex justify-end items-center">
        <SelectPeriod
          options={options}
          selected={period}
          onChange={setPeriod}
        />
      </div>

      <ProcessTable period={period} />
    </div>
  );
};
