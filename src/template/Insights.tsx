import { InsightChart } from "@/components/charts/insights/InsightChart";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { archivePeriods } from "@/consts";
import { useTauriStore } from "@/hooks/useTauriStore";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";

const Border = ({ children }: { children: JSX.Element }) => {
  return (
    <div className="border rounded-2xl border-zinc-400 dark:border-zinc-600 p-4">
      {children}
    </div>
  );
};

const SelectPeriod = ({
  options,
  selected,
  onChange,
  showDefaultOption,
}: {
  options: { label: string; value: keyof typeof archivePeriods }[];
  selected: keyof typeof archivePeriods | "all";
  onChange: (value: (typeof archivePeriods)[number]) => void;
  showDefaultOption?: boolean;
}) => {
  const { t } = useTranslation();

  return (
    <Select
      value={String(selected)}
      onValueChange={(value) =>
        onChange(value as unknown as (typeof archivePeriods)[number])
      }
    >
      <SelectTrigger className="w-[180px]">
        <SelectValue placeholder="Temperature Unit" />
      </SelectTrigger>
      <SelectContent>
        {showDefaultOption && (
          <SelectItem key="all" value="all">
            {t("shared.select")}
          </SelectItem>
        )}
        {options.map((option) => (
          <SelectItem key={String(option.value)} value={String(option.value)}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
};

export const Insights = () => {
  const { t } = useTranslation();
  const [periodAvgCPU, setPeriodAvgCPU] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodAvgCPU", 180);
  const [periodAvgRAM, setPeriodAvgRAM] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodAvgRAM", 180);
  const [periodMaxCPU, setPeriodMaxCPU] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMaxCPU", 180);
  const [periodMaxRAM, setPeriodMaxRAM] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMaxRAM", 180);
  const [periodMinCPU, setPeriodMinCPU] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMinCPU", 180);
  const [periodMinRAM, setPeriodMinRAM] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMinRAM", 180);

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

  const selections = [
    periodAvgCPU,
    periodAvgRAM,
    periodMaxCPU,
    periodMaxRAM,
    periodMinCPU,
    periodMinRAM,
  ];

  return (
    <div className="pb-6">
      <div className="flex justify-end items-center">
        <SelectPeriod
          options={options}
          selected={
            selections.every((s) => s === selections[0]) ? periodAvgCPU : "all"
          }
          onChange={(v) => {
            setPeriodAvgCPU(v);
            setPeriodAvgRAM(v);
            setPeriodMaxCPU(v);
            setPeriodMaxRAM(v);
            setPeriodMinCPU(v);
            setPeriodMinRAM(v);
          }}
          showDefaultOption={!selections.every((s) => s === selections[0])}
        />
      </div>
      <div className="grid grid-cols-2 gap-6 mt-6">
        <Border>
          <>
            <div className="flex justify-between items-center">
              <h3 className="text-2xl font-bold py-3">
                {t("shared.cpuUsage")} ({t("shared.avg")})
              </h3>
              <SelectPeriod
                options={options}
                selected={periodAvgCPU}
                onChange={setPeriodAvgCPU}
              />
            </div>

            <InsightChart dataType="cpu" period={periodAvgCPU} type="cpu_avg" />
          </>
        </Border>
        <Border>
          <>
            <div className="flex justify-between items-center">
              <h3 className="text-2xl font-bold py-3">
                {t("shared.memoryUsage")} ({t("shared.avg")})
              </h3>
              <SelectPeriod
                options={options}
                selected={periodAvgRAM}
                onChange={setPeriodAvgRAM}
              />
            </div>
            <InsightChart
              dataType="memory"
              period={periodAvgRAM}
              type="ram_avg"
            />
          </>
        </Border>
        <Border>
          <>
            <div className="flex justify-between items-center">
              <h3 className="text-2xl font-bold py-3">
                {t("shared.cpuUsage")} ({t("shared.max")})
              </h3>
              <SelectPeriod
                options={options}
                selected={periodMaxCPU}
                onChange={setPeriodMaxCPU}
              />
            </div>

            <InsightChart dataType="cpu" period={periodMaxCPU} type="cpu_max" />
          </>
        </Border>
        <Border>
          <>
            <div className="flex justify-between items-center">
              <h3 className="text-2xl font-bold py-3">
                {t("shared.memoryUsage")} ({t("shared.max")})
              </h3>
              <SelectPeriod
                options={options}
                selected={periodMaxRAM}
                onChange={setPeriodMaxRAM}
              />
            </div>
            <InsightChart
              dataType="memory"
              period={periodMinRAM}
              type="ram_max"
            />
          </>
        </Border>
        <Border>
          <>
            <div className="flex justify-between items-center">
              <h3 className="text-2xl font-bold py-3">
                {t("shared.cpuUsage")} ({t("shared.min")})
              </h3>
              <SelectPeriod
                options={options}
                selected={periodMinCPU}
                onChange={setPeriodMinCPU}
              />
            </div>
            <InsightChart dataType="cpu" period={periodMinCPU} type="cpu_min" />
          </>
        </Border>
        <Border>
          <>
            <div className="flex justify-between items-center">
              <h3 className="text-2xl font-bold py-3">
                {t("shared.memoryUsage")} ({t("shared.min")})
              </h3>
              <SelectPeriod
                options={options}
                selected={periodMinRAM}
                onChange={setPeriodMinRAM}
              />
            </div>
            <InsightChart
              dataType="memory"
              period={periodMinRAM}
              type="ram_min"
            />
          </>
        </Border>
      </div>
    </div>
  );
};
