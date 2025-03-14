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
import type { ChartDataType, DataStats } from "@/types/hardwareDataType";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { type JSX, useState } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";

const arrowButtonVariants = tv({
  base: "text-zinc-500 dark:text-zinc-400 cursor-pointer disabled:opacity-50 disabled:pointer-events-none h-40",
});

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
  selected: keyof typeof archivePeriods | null;
  onChange: (value: (typeof archivePeriods)[number]) => void;
  showDefaultOption?: boolean;
}) => {
  const { t } = useTranslation();

  return (
    <Select
      value={String(selected ?? "not-selected")}
      onValueChange={(value) =>
        onChange(value as unknown as (typeof archivePeriods)[number])
      }
    >
      <SelectTrigger className="w-[180px]">
        <SelectValue placeholder="Temperature Unit" />
      </SelectTrigger>
      <SelectContent>
        {showDefaultOption && (
          <SelectItem key="" value="not-selected">
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
    (typeof archivePeriods)[number] | null
  >("periodAvgCPU", null);
  const [periodAvgRAM, setPeriodAvgRAM] = useTauriStore<
    (typeof archivePeriods)[number] | null
  >("periodAvgRAM", null);
  const [periodMaxCPU, setPeriodMaxCPU] = useTauriStore<
    (typeof archivePeriods)[number] | null
  >("periodMaxCPU", null);
  const [periodMaxRAM, setPeriodMaxRAM] = useTauriStore<
    (typeof archivePeriods)[number] | null
  >("periodMaxRAM", null);
  const [periodMinCPU, setPeriodMinCPU] = useTauriStore<
    (typeof archivePeriods)[number] | null
  >("periodMinCPU", null);
  const [periodMinRAM, setPeriodMinRAM] = useTauriStore<
    (typeof archivePeriods)[number] | null
  >("periodMinRAM", null);

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

  const chartData: {
    type: Exclude<ChartDataType, "gpu">;
    stats: DataStats;
    period: [
      (typeof archivePeriods)[number] | null,
      (newValue: (typeof archivePeriods)[number] | null) => Promise<void>,
    ];
  }[] = [
    { type: "cpu", stats: "avg", period: [periodAvgCPU, setPeriodAvgCPU] },
    { type: "memory", stats: "avg", period: [periodAvgRAM, setPeriodAvgRAM] },
    { type: "cpu", stats: "max", period: [periodMaxCPU, setPeriodMaxCPU] },
    { type: "memory", stats: "max", period: [periodMaxRAM, setPeriodMaxRAM] },
    { type: "cpu", stats: "min", period: [periodMinCPU, setPeriodMinCPU] },
    { type: "memory", stats: "min", period: [periodMinRAM, setPeriodMinRAM] },
  ];

  return (
    <div className="pb-6">
      <div className="flex justify-end items-center">
        <SelectPeriod
          options={options}
          selected={
            selections.every((s) => s === selections[0]) ? periodAvgCPU : null
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
        {chartData.map((data) => {
          const { type, stats, period } = data;
          const [periodData, setPeriodData] = period;
          const [offset, setOffset] = useState(0);
          const [intervalId, setIntervalId] = useState<NodeJS.Timeout | null>(
            null,
          );

          const handleMouseDown = (increment: number) => {
            if (intervalId) return;
            const id = setInterval(() => {
              setOffset((prev) => Math.max(0, prev + increment));
            }, 100);
            setIntervalId(id);
          };

          const handleMouseUp = () => {
            if (intervalId) {
              clearInterval(intervalId);
              setIntervalId(null);
            }
          };

          return (
            periodData && (
              <Border key={`${data.type}-${data.stats}`}>
                <>
                  <div className="flex justify-between items-center">
                    <h3 className="text-2xl font-bold py-3">
                      {t(`shared.${data.type}Usage`)} (
                      {t(`shared.${data.stats}`)})
                    </h3>
                    <SelectPeriod
                      options={options}
                      selected={periodData}
                      onChange={setPeriodData}
                    />
                  </div>

                  <div className="flex justify-between items-center">
                    <button
                      type="button"
                      className={arrowButtonVariants()}
                      onClick={() => setOffset(offset + 1)}
                      onMouseDown={() => handleMouseDown(1)}
                      onMouseUp={handleMouseUp}
                      onMouseLeave={handleMouseUp}
                      onTouchStart={() => handleMouseDown(1)}
                      onTouchEnd={handleMouseUp}
                    >
                      <ChevronLeft size={32} />
                    </button>
                    <InsightChart
                      hardwareType={type}
                      period={periodData}
                      dataStats={stats}
                      offset={offset}
                    />
                    <button
                      type="button"
                      className={arrowButtonVariants()}
                      onClick={() => setOffset(offset - 1)}
                      onMouseDown={() => handleMouseDown(-1)}
                      onMouseUp={handleMouseUp}
                      onMouseLeave={handleMouseUp}
                      onTouchStart={() => handleMouseDown(-1)}
                      onTouchEnd={handleMouseUp}
                      disabled={offset < 0}
                    >
                      <ChevronRight size={32} />
                    </button>
                  </div>
                </>
              </Border>
            )
          );
        })}
      </div>
    </div>
  );
};
