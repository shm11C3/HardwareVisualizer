import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { archivePeriods } from "@/features/hardware/consts/chart";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import {
  GpuInsightChart,
  InsightChart,
} from "@/features/hardware/insights/components/InsightChart";
import type {
  ChartDataType,
  DataStats,
  GpuDataType,
} from "@/features/hardware/types/hardwareDataType";
import { useTauriStore } from "@/hooks/useTauriStore";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { type JSX, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { tv } from "tailwind-variants";
import { useGpuNames } from "../hooks/useGpuNames";
import { SelectPeriod } from "./components/SelectPeriod";
import { ProcessInsight } from "./process/ProcessInsight";

const arrowButtonVariants = tv({
  base: "text-zinc-500 dark:text-zinc-400 cursor-pointer disabled:opacity-50 disabled:pointer-events-none h-40",
});

const Border = ({ children }: { children: JSX.Element }) => {
  return (
    <div className="rounded-2xl border border-zinc-400 p-4 dark:border-zinc-600">
      {children}
    </div>
  );
};

const MainInsights = () => {
  const { t } = useTranslation();
  const [periodAvgCPU, setPeriodAvgCPU] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodAvgCPU", 60);
  const [periodAvgRAM, setPeriodAvgRAM] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodAvgRAM", 60);
  const [periodMaxCPU, setPeriodMaxCPU] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMaxCPU", 60);
  const [periodMaxRAM, setPeriodMaxRAM] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMaxRAM", 60);
  const [periodMinCPU, setPeriodMinCPU] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMinCPU", 60);
  const [periodMinRAM, setPeriodMinRAM] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMinRAM", 60);

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
      (newValue: (typeof archivePeriods)[number]) => Promise<void>,
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
      <div className="flex items-center justify-end">
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

      <div className="mt-6 grid grid-cols-2 gap-6">
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
                  <div className="flex items-center justify-between">
                    <h3 className="py-3 font-bold text-2xl">
                      {t(`shared.${data.type}Usage`)} (
                      {t(`shared.${data.stats}`)})
                    </h3>
                    <SelectPeriod
                      options={options}
                      selected={periodData}
                      onChange={setPeriodData}
                    />
                  </div>

                  <div className="flex items-center justify-between">
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

const GPUInsights = ({ gpuName }: { gpuName: string }) => {
  const { t } = useTranslation();
  const [periodAvgGpuUsage, setPeriodAvgGpuUsage] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodAvgGpuUsage", 60);
  const [periodAvgGpuTemperature, setPeriodAvgGpuTemperature] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodAvgGpuTemperature", 60);
  const [periodMaxGpuUsage, setPeriodMaxGpuUsage] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMaxGpuUsage", 60);
  const [periodMaxGpuTemperature, setPeriodMaxGpuTemperature] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMaxGpuTemperature", 60);
  const [periodMinGpuUsage, setPeriodMinGpuUsage] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMinGpuUsage", 60);
  const [periodMinGpuTemperature, setPeriodMinGpuTemperature] = useTauriStore<
    (typeof archivePeriods)[number]
  >("periodMinGpuTemperature", 60);
  const [periodAvgGpuDedicatedMemory, setPeriodAvgGpuDedicatedMemory] =
    useTauriStore<(typeof archivePeriods)[number]>(
      "periodAvgGpuDedicatedMemory",
      60,
    );
  const [periodMaxGpuDedicatedMemory, setPeriodMaxGpuDedicatedMemory] =
    useTauriStore<(typeof archivePeriods)[number]>(
      "periodMaxGpuDedicatedMemory",
      60,
    );
  const [periodMinGpuDedicatedMemory, setPeriodMinGpuDedicatedMemory] =
    useTauriStore<(typeof archivePeriods)[number]>(
      "periodMinGpuDedicatedMemory",
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

  const selections = [
    periodAvgGpuUsage,
    periodAvgGpuTemperature,
    periodMaxGpuUsage,
    periodMaxGpuTemperature,
    periodMinGpuUsage,
    periodMinGpuTemperature,
  ];

  const chartData: {
    type: GpuDataType;
    stats: DataStats;
    period: [
      (typeof archivePeriods)[number] | null,
      (newValue: (typeof archivePeriods)[number]) => Promise<void>,
    ];
  }[] = [
    {
      type: "usage",
      stats: "avg",
      period: [periodAvgGpuUsage, setPeriodAvgGpuUsage],
    },
    {
      type: "dedicatedMemory",
      stats: "avg",
      period: [periodAvgGpuDedicatedMemory, setPeriodAvgGpuDedicatedMemory],
    },
    {
      type: "usage",
      stats: "max",
      period: [periodMaxGpuUsage, setPeriodMaxGpuUsage],
    },
    {
      type: "dedicatedMemory",
      stats: "max",
      period: [periodMaxGpuDedicatedMemory, setPeriodMaxGpuDedicatedMemory],
    },
    {
      type: "usage",
      stats: "min",
      period: [periodMinGpuUsage, setPeriodMinGpuUsage],
    },
    {
      type: "dedicatedMemory",
      stats: "min",
      period: [periodMinGpuDedicatedMemory, setPeriodMinGpuDedicatedMemory],
    },
    {
      type: "temp",
      stats: "avg",
      period: [periodAvgGpuTemperature, setPeriodAvgGpuTemperature],
    },
    {
      type: "temp",
      stats: "max",
      period: [periodMaxGpuTemperature, setPeriodMaxGpuTemperature],
    },

    {
      type: "temp",
      stats: "min",
      period: [periodMinGpuTemperature, setPeriodMinGpuTemperature],
    },
  ];

  return (
    <div className="pb-6">
      <div className="flex items-center justify-end">
        <SelectPeriod
          options={options}
          selected={
            selections.every((s) => s === selections[0])
              ? periodAvgGpuUsage
              : null
          }
          onChange={(v) => {
            setPeriodAvgGpuUsage(v);
            setPeriodAvgGpuTemperature(v);
            setPeriodMaxGpuUsage(v);
            setPeriodMaxGpuTemperature(v);
            setPeriodMinGpuUsage(v);
            setPeriodMinGpuTemperature(v);
            setPeriodAvgGpuDedicatedMemory(v);
            setPeriodMaxGpuDedicatedMemory(v);
            setPeriodMinGpuDedicatedMemory(v);
          }}
          showDefaultOption={!selections.every((s) => s === selections[0])}
        />
      </div>

      <div className="mt-6 grid grid-cols-2 gap-6">
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

          const dataType: Record<
            "temp" | "usage" | "dedicatedMemory",
            "usage" | "temperature" | "memorySizeDedicatedUsage"
          > = {
            usage: "usage",
            temp: "temperature",
            dedicatedMemory: "memorySizeDedicatedUsage",
          };

          const dataTypeKeys = dataType[data.type];

          return (
            periodData && (
              <Border key={`${data.type}-${data.stats}`}>
                <>
                  <div className="flex items-center justify-between">
                    <h3 className="py-3 font-bold text-2xl">
                      {t(`shared.${dataTypeKeys}`)} ({t(`shared.${data.stats}`)}
                      )
                    </h3>
                    <SelectPeriod
                      options={options}
                      selected={periodData}
                      onChange={setPeriodData}
                    />
                  </div>

                  <div className="flex items-center justify-between">
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
                    <GpuInsightChart
                      dataType={type}
                      period={periodData}
                      dataStats={stats}
                      offset={offset}
                      gpuName={gpuName}
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

export const Insights = () => {
  const { t } = useTranslation();
  const gpuNames = useGpuNames();
  const { init } = useHardwareInfoAtom();
  const [displayTarget, setDisplayTarget, isPending] = useTauriStore<string>(
    "insightDisplayTarget",
    "main",
  );

  useEffect(() => {
    init();
  }, [init]);

  const insightsChild: {
    key: string;
    element: JSX.Element;
  }[] = [
    { key: "main", element: <MainInsights /> },
    ...(gpuNames.length
      ? gpuNames.map((v) => {
          return {
            key: v,
            element: <GPUInsights gpuName={v} />,
          };
        })
      : []),
    { key: "process", element: <ProcessInsight /> },
  ];

  return (
    !isPending && (
      <Tabs
        value={
          insightsChild.some((v) => v.key === displayTarget)
            ? displayTarget
            : "main"
        }
      >
        {insightsChild.length > 1 && (
          <TabsList>
            {insightsChild.map((child) => {
              const { key } = child;
              return (
                <TabsTrigger
                  key={key}
                  value={key}
                  onClick={() => setDisplayTarget(key)}
                >
                  {["main", "process"].includes(key)
                    ? t(`pages.insights.${key}.title`, { defaultValue: key })
                    : key}
                </TabsTrigger>
              );
            })}
          </TabsList>
        )}
        {insightsChild.map(({ key, element }) => (
          <TabsContent key={key} value={key}>
            {element}
          </TabsContent>
        ))}
      </Tabs>
    )
  );
};
