import { useAtomValue } from "jotai";
import { type CSSProperties, memo, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  cpuTempAtom,
  cpuUsageHistoryAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  memoryUsageHistoryAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";

const toCssColor = (value: string) =>
  value.startsWith("rgb(") ? value : `rgb(${value})`;

const formatCurrentValue = (value: number | null | undefined) =>
  value == null ? "—" : `${Math.round(value)}%`;

const formatTemperature = (
  value: number | null | undefined,
  unit: "C" | "F",
) => {
  if (value == null) {
    return undefined;
  }

  const converted = unit === "F" ? (value * 9) / 5 + 32 : value;
  return `${Math.round(converted)}°${unit}`;
};

const Sparkline = ({
  values,
  color,
}: {
  values: (number | null)[];
  color: string;
}) => {
  const segments = useMemo(() => {
    const width = 180;
    const height = 48;
    const denominator = Math.max(values.length - 1, 1);
    const nextSegments: Array<{ startIndex: number; points: string[] }> = [];

    values.forEach((value, index) => {
      if (value == null) {
        return;
      }

      const point = `${(index / denominator) * width},${
        height - (Math.min(100, Math.max(0, value)) / 100) * height
      }`;
      const previousValue = values[index - 1];
      if (index === 0 || previousValue == null) {
        nextSegments.push({ startIndex: index, points: [point] });
        return;
      }

      nextSegments.at(-1)?.points.push(point);
    });

    return nextSegments.map(({ startIndex, points }) => ({
      startIndex,
      points: points.join(" "),
    }));
  }, [values]);

  return (
    <svg
      viewBox="0 0 180 48"
      preserveAspectRatio="none"
      aria-hidden="true"
      className="h-12 w-full overflow-visible"
    >
      <path
        d="M0 47.5H180"
        stroke="currentColor"
        strokeOpacity="0.12"
        vectorEffect="non-scaling-stroke"
      />
      {segments.map(({ startIndex, points }) => (
        <polyline
          key={startIndex}
          points={points}
          fill="none"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          vectorEffect="non-scaling-stroke"
        />
      ))}
    </svg>
  );
};

const MetricSignal = memo(
  ({
    label,
    metricId,
    history,
    color,
    temperature,
  }: {
    label: string;
    metricId: "cpu" | "memory" | "gpu";
    history: (number | null)[];
    color: string;
    temperature?: string | undefined;
  }) => {
    const { t } = useTranslation();
    const currentValue = history.at(-1);

    return (
      <article
        className="relative min-w-0 overflow-hidden rounded-xl border border-border bg-card/80 p-4 shadow-sm"
        style={{ "--metric-color": color } as CSSProperties}
        data-testid={`performance-metric-${metricId}`}
      >
        <div
          className="absolute inset-x-0 top-0 h-0.5 bg-[var(--metric-color)]"
          aria-hidden="true"
        />
        <div className="flex items-start justify-between gap-3">
          <div>
            <p className="font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
              {label}
            </p>
            <p className="mt-1 font-mono font-semibold text-3xl tabular-nums tracking-tight">
              {formatCurrentValue(currentValue)}
            </p>
          </div>
          {temperature != null && (
            <p className="rounded-full bg-muted px-2 py-1 font-mono text-xs tabular-nums">
              {temperature}
            </p>
          )}
        </div>
        <div className="mt-2 text-muted-foreground">
          <Sparkline values={history} color={color} />
        </div>
        <p className="mt-1 text-muted-foreground text-xs">
          {t("pages.performance.shortWindow")}
        </p>
      </article>
    );
  },
);

export const CurrentValueStrip = ({ className }: { className?: string }) => {
  const { t } = useTranslation();
  const cpuHistory = useAtomValue(cpuUsageHistoryAtom);
  const memoryHistory = useAtomValue(memoryUsageHistoryAtom);
  const gpuUsageHistories = useAtomValue(gpuUsageHistoriesAtom);
  const cpuTemperatures = useAtomValue(cpuTempAtom);
  const gpuTemperatureMap = useAtomValue(gpuTempMapAtom);
  const selectedGpuId = useAtomValue(selectedGpuIdAtom);
  const { settings } = useSettingsAtom();
  const effectiveGpuId =
    selectedGpuId != null && Object.hasOwn(gpuUsageHistories, selectedGpuId)
      ? selectedGpuId
      : (Object.keys(gpuUsageHistories)[0] ??
        Object.keys(gpuTemperatureMap)[0]);
  const gpuHistory =
    effectiveGpuId != null ? (gpuUsageHistories[effectiveGpuId] ?? []) : [];
  const gpuTemperature =
    effectiveGpuId != null
      ? gpuTemperatureMap[effectiveGpuId]?.value
      : undefined;

  return (
    <section
      className={cn("grid gap-3 md:grid-cols-3", className)}
      aria-label={t("pages.performance.panels.currentValues")}
      data-testid="performance-current-values"
    >
      <MetricSignal
        metricId="cpu"
        label={t("pages.performance.metrics.cpu")}
        history={cpuHistory}
        color={toCssColor(settings.lineGraphColor.cpu)}
        temperature={formatTemperature(
          cpuTemperatures[0]?.value,
          settings.temperatureUnit,
        )}
      />
      <MetricSignal
        metricId="memory"
        label={t("pages.performance.metrics.memory")}
        history={memoryHistory}
        color={toCssColor(settings.lineGraphColor.memory)}
      />
      <MetricSignal
        metricId="gpu"
        label={t("pages.performance.metrics.gpu")}
        history={gpuHistory}
        color={toCssColor(settings.lineGraphColor.gpu)}
        temperature={formatTemperature(
          gpuTemperature,
          settings.temperatureUnit,
        )}
      />
    </section>
  );
};
