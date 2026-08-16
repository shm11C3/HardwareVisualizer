import { CpuIcon, GraphicsCardIcon, MemoryIcon } from "@phosphor-icons/react";
import { useAtom, useAtomValue } from "jotai";
import {
  type CSSProperties,
  memo,
  type ReactNode,
  useEffect,
  useMemo,
} from "react";
import { useTranslation } from "react-i18next";
import { DoughnutChart } from "@/components/charts/DoughnutChart";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import {
  cpuTempAtom,
  cpuUsageHistoryAtom,
  gpuDedicatedMemoryKbMapAtom,
  gpuFanSpeedMapAtom,
  gpuTempMapAtom,
  gpuUsageHistoriesAtom,
  memoryUsageHistoryAtom,
  processorsUsageHistoryAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useWindowSize } from "@/hooks/useWindowSize";
import { cn } from "@/lib/utils";
import {
  getEffectiveGpuId,
  hasNoLiveGpuReadings,
  listGpuAdapters,
} from "../gpuIdentity";
import { GpuAdapterSelector } from "./GpuAdapterSelector";
import { Sparkline } from "./Sparkline";

export const toCssColor = (value: string) =>
  value.startsWith("rgb(") ? value : `rgb(${value})`;

export const formatTemperature = (
  value: number | null | undefined,
  unit: "C" | "F",
) => {
  if (value == null) {
    return undefined;
  }

  const converted = unit === "F" ? (value * 9) / 5 + 32 : value;
  return `${Math.round(converted)}°${unit}`;
};

type Substat = { key: string; text: string };

const MetricInstrument = memo(
  ({
    label,
    metricId,
    history,
    color,
    badge,
    identity,
    note,
    substats,
    gauges,
    icon,
    staggered = false,
  }: {
    label: string;
    metricId: "cpu" | "memory" | "gpu";
    history: (number | null)[];
    color: string;
    badge?: string | undefined;
    /** Names the physical device the readings came from, e.g. the GPU adapter. */
    identity?: ReactNode;
    /** Stated instead of substats when the device reports nothing. */
    note?: string | undefined;
    substats: Substat[];
    /** Classic Hardware Dashboard card icon, colored per metric hue. */
    icon: ReactNode;
    /** Classic Hardware Dashboard doughnut charts (usage plus optional temp). */
    gauges: ReactNode;
    /** Classic narrow-width trick: taller row for diagonally shifted gauges. */
    staggered?: boolean;
  }) => (
    <article
      className="relative min-w-0 overflow-hidden rounded-2xl bg-card p-4 pb-3"
      style={{ "--metric-color": color } as CSSProperties}
      data-testid={`performance-metric-${metricId}`}
    >
      <div
        className="absolute inset-x-0 top-0 h-0.5 bg-[var(--metric-color)]"
        aria-hidden="true"
      />
      <div className="flex min-w-0 items-center gap-2">
        {icon}
        <p className="shrink-0 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
          {label}
        </p>
        {badge != null && (
          <p className="ml-auto rounded-full bg-muted px-2 py-0.5 font-mono text-xs tabular-nums">
            {badge}
          </p>
        )}
        {identity != null && <div className="ml-auto min-w-0">{identity}</div>}
      </div>
      {/* Two side-by-side doughnuts must fit a one-third-width card, so the
          xl row stays below the classic 200px dashboard height. */}
      <div
        className={cn(
          "mt-2 flex justify-around",
          staggered ? "h-[150px]" : "h-[100px] xl:h-[160px]",
        )}
      >
        {gauges}
      </div>
      <div className="mt-2 text-muted-foreground">
        <Sparkline values={history} color={color} />
      </div>
      {note != null ? (
        <p className="mt-1.5 text-[11px] text-muted-foreground">{note}</p>
      ) : (
        substats.length > 0 && (
          <div className="mt-1.5 flex flex-wrap gap-x-3 gap-y-0.5 font-mono text-[11px] text-muted-foreground tabular-nums">
            {substats.map((substat) => (
              <span key={substat.key}>{substat.text}</span>
            ))}
          </div>
        )
      )}
    </article>
  ),
);

/**
 * Fixed live header of the Performance panels view: one hue-coded instrument
 * per top-level metric, reusing the classic Hardware Dashboard doughnut
 * charts (usage plus a temperature doughnut where the platform reports one).
 * Secondary readings render only when the platform actually reports them.
 */
export const InstrumentStrip = ({ className }: { className?: string }) => {
  const { t } = useTranslation();
  const cpuHistory = useAtomValue(cpuUsageHistoryAtom);
  const memoryHistory = useAtomValue(memoryUsageHistoryAtom);
  const gpuUsageHistories = useAtomValue(gpuUsageHistoriesAtom);
  const cpuTemperatures = useAtomValue(cpuTempAtom);
  const gpuTemperatureMap = useAtomValue(gpuTempMapAtom);
  const gpuFanSpeedMap = useAtomValue(gpuFanSpeedMapAtom);
  const gpuDedicatedMemoryKbMap = useAtomValue(gpuDedicatedMemoryKbMapAtom);
  const processorsUsageHistory = useAtomValue(processorsUsageHistoryAtom);
  const [selectedGpuId, setSelectedGpuId] = useAtom(selectedGpuIdAtom);
  const { settings } = useSettingsAtom();
  const { hardwareInfo, init } = useHardwareInfoAtom();
  const { isBreak } = useWindowSize();
  // Classic GPUInfo narrow-width trick: when the three-column strip leaves a
  // card too narrow for two side-by-side doughnuts, shift the temperature
  // gauge down so the pair overlaps diagonally instead of clipping.
  const staggerGauges = isBreak("md") && !isBreak("lg");

  // biome-ignore lint/correctness/useExhaustiveDependencies: one-time static-fact fetch
  useEffect(() => {
    void init();
  }, []);

  const gpuAdapters = useMemo(
    () =>
      listGpuAdapters(
        hardwareInfo.gpus,
        gpuTemperatureMap,
        Object.keys(gpuUsageHistories),
      ),
    [hardwareInfo.gpus, gpuTemperatureMap, gpuUsageHistories],
  );
  const effectiveGpuId = getEffectiveGpuId(
    selectedGpuId,
    gpuUsageHistories,
    gpuTemperatureMap,
    gpuAdapters.map((adapter) => adapter.id),
  );
  const gpuHasNoReadings = hasNoLiveGpuReadings(
    effectiveGpuId,
    gpuUsageHistories,
    gpuTemperatureMap,
  );
  const gpuHistory =
    effectiveGpuId != null ? (gpuUsageHistories[effectiveGpuId] ?? []) : [];
  const gpuTemperature =
    effectiveGpuId != null
      ? gpuTemperatureMap[effectiveGpuId]?.value
      : undefined;
  const cpuTemperature = cpuTemperatures[0]?.value;

  const cpuSubstats = useMemo<Substat[]>(() => {
    if (hardwareInfo.cpu == null) {
      return [];
    }

    const threadCount = processorsUsageHistory.at(-1)?.length ?? 0;
    return [
      {
        key: "clock",
        text: `${hardwareInfo.cpu.clock} ${hardwareInfo.cpu.clockUnit}`,
      },
      {
        key: "cores",
        text:
          threadCount > 0
            ? `${hardwareInfo.cpu.coreCount}C/${threadCount}T`
            : `${hardwareInfo.cpu.coreCount}C`,
      },
    ];
  }, [hardwareInfo.cpu, processorsUsageHistory]);

  const memoryReadings = useMemo(() => {
    const current = memoryHistory.at(-1);
    const [total, unit] = hardwareInfo.memory?.size.split(" ") ?? [null, null];
    if (current == null || total == null || unit == null) {
      return { badge: undefined, usedValue: null, usedUnit: null };
    }

    const used = Number(
      ((current / 100) * Number.parseFloat(total)).toFixed(0),
    );
    return {
      badge: `${used} / ${total} ${unit}`,
      usedValue: used,
      // Report the unit the platform gave rather than assuming MB for
      // anything that is not GB.
      usedUnit: unit,
    };
  }, [memoryHistory, hardwareInfo.memory]);

  const memorySubstats = useMemo<Substat[]>(() => {
    if (hardwareInfo.memory == null) {
      return [];
    }

    const substats: Substat[] = [
      { key: "type", text: hardwareInfo.memory.memoryType },
    ];
    if (hardwareInfo.memory.isDetailed && hardwareInfo.memory.clock > 0) {
      substats.push({
        key: "clock",
        text: `${hardwareInfo.memory.clock} ${hardwareInfo.memory.clockUnit}`,
      });
    }
    return substats;
  }, [hardwareInfo.memory]);

  const gpuSubstats = useMemo<Substat[]>(() => {
    const substats: Substat[] = [];
    if (effectiveGpuId != null) {
      const usedKb = gpuDedicatedMemoryKbMap[effectiveGpuId];
      if (usedKb != null) {
        const totalLabel = hardwareInfo.gpus?.find(
          (gpu) => gpu.id === effectiveGpuId,
        )?.memorySizeDedicated;
        const usedGb = (usedKb / 1024 / 1024).toFixed(1);
        substats.push({
          key: "vram",
          text:
            totalLabel != null && totalLabel !== "N/A"
              ? `VRAM ${usedGb}/${totalLabel}`
              : `VRAM ${usedGb} GB`,
        });
      }

      const fan = gpuFanSpeedMap[effectiveGpuId];
      if (fan != null) {
        substats.push({
          key: "fan",
          // VRAM stays an acronym in every supported language; "fan" is a
          // word, so it comes from the language files.
          text: t("pages.performance.substats.fan", {
            value: Math.round(fan.value),
          }),
        });
      }
    }
    return substats;
  }, [
    effectiveGpuId,
    gpuDedicatedMemoryKbMap,
    gpuFanSpeedMap,
    hardwareInfo.gpus,
    t,
  ]);

  const currentMemoryUsage = memoryHistory.at(-1) ?? null;

  return (
    <section
      className={cn("grid gap-3 md:grid-cols-3", className)}
      aria-label={t("pages.performance.currentValues")}
      data-testid="performance-current-values"
    >
      <MetricInstrument
        metricId="cpu"
        label={t("pages.performance.metrics.cpu")}
        history={cpuHistory}
        color={toCssColor(settings.lineGraphColor.cpu)}
        substats={cpuSubstats}
        icon={
          <CpuIcon size={22} color={`rgb(${settings.lineGraphColor.cpu})`} />
        }
        staggered={staggerGauges && cpuTemperature != null}
        gauges={
          <>
            <DoughnutChart
              chartValue={cpuHistory.at(-1) ?? null}
              dataType="usage"
            />
            {cpuTemperature != null && (
              <DoughnutChart
                chartValue={cpuTemperature}
                dataType="temp"
                className={staggerGauges ? "mt-12" : ""}
              />
            )}
          </>
        }
      />
      <MetricInstrument
        metricId="memory"
        label={t("pages.performance.metrics.memory")}
        history={memoryHistory}
        color={toCssColor(settings.lineGraphColor.memory)}
        badge={memoryReadings.badge}
        substats={memorySubstats}
        icon={
          <MemoryIcon
            size={22}
            color={`rgb(${settings.lineGraphColor.memory})`}
          />
        }
        gauges={
          memoryReadings.usedValue != null &&
          memoryReadings.usedUnit != null ? (
            <DoughnutChart
              chartValue={memoryReadings.usedValue}
              usagePercentage={currentMemoryUsage ?? 0}
              dataType="memoryUsageValue"
              unit={memoryReadings.usedUnit}
            />
          ) : (
            <DoughnutChart chartValue={currentMemoryUsage} dataType="usage" />
          )
        }
      />
      <MetricInstrument
        metricId="gpu"
        label={t("pages.performance.metrics.gpu")}
        history={gpuHistory}
        color={toCssColor(settings.lineGraphColor.gpu)}
        substats={gpuSubstats}
        note={
          gpuHasNoReadings
            ? t("pages.performance.gpuNoLiveReadings")
            : undefined
        }
        identity={
          <GpuAdapterSelector
            adapters={gpuAdapters}
            selectedId={effectiveGpuId}
            onSelect={setSelectedGpuId}
          />
        }
        icon={
          <GraphicsCardIcon
            size={22}
            color={`rgb(${settings.lineGraphColor.gpu})`}
          />
        }
        staggered={staggerGauges && gpuTemperature != null}
        gauges={
          <>
            <DoughnutChart
              chartValue={gpuHistory.at(-1) ?? null}
              dataType="usage"
            />
            {gpuTemperature != null && (
              <DoughnutChart
                chartValue={gpuTemperature}
                dataType="temp"
                className={staggerGauges ? "mt-12" : ""}
              />
            )}
          </>
        }
      />
    </section>
  );
};
