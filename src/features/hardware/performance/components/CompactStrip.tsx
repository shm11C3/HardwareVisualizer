import { useAtomValue } from "jotai";
import { type CSSProperties, memo } from "react";
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
import {
  formatTemperature,
  getEffectiveGpuId,
  toCssColor,
} from "./InstrumentStrip";
import { Sparkline } from "./Sparkline";

/**
 * One Compact line. The strip renders whatever rows the platform can feed it,
 * so metrics that are not collected yet (disk activity, network throughput)
 * become new builders here instead of new layout work.
 */
export type CompactMetricRow = {
  id: string;
  label: string;
  color: string;
  history: (number | null)[];
  /** Short right-aligned reading next to the percent, e.g. temperature. */
  detail?: string | undefined;
};

const CompactRow = memo(
  ({ row, expanded }: { row: CompactMetricRow; expanded: boolean }) => {
    const currentValue = row.history.at(-1) ?? null;
    const barWidth =
      currentValue == null ? 0 : Math.min(100, Math.max(0, currentValue));

    return (
      <div
        className={cn(
          "grid items-center border-border/60 border-b last:border-b-0",
          expanded
            ? "min-h-0 flex-1 grid-cols-[5rem_minmax(6rem,1fr)_6rem_4.5rem_minmax(12rem,2.6fr)] gap-4 py-3"
            : "grid-cols-[2.8rem_minmax(3.5rem,1fr)_2.8rem_2.8rem_minmax(5rem,1.2fr)] gap-2.5 py-3",
        )}
        style={{ "--metric-color": row.color } as CSSProperties}
        data-testid={`performance-compact-row-${row.id}`}
      >
        <span
          className={cn(
            "font-mono font-semibold text-muted-foreground uppercase tracking-[0.1em]",
            expanded ? "text-base" : "text-[11px]",
          )}
        >
          {row.label}
        </span>
        <span
          className={cn(
            "overflow-hidden rounded-full bg-muted",
            expanded ? "h-2.5" : "h-1.5",
          )}
          aria-hidden="true"
        >
          {/* scaleX stays on the compositor; animating width would rerun
              layout for every frame of every 1 Hz tick (see the doughnut
              tween note in DoughnutChart). */}
          <span
            className="block h-full w-full origin-left rounded-full bg-[var(--metric-color)] transition-transform duration-300 ease-out motion-reduce:transition-none"
            style={{ transform: `scaleX(${barWidth / 100})` }}
          />
        </span>
        <span
          className={cn(
            "text-right font-mono tabular-nums",
            expanded ? "font-semibold text-2xl" : "text-sm",
          )}
        >
          {currentValue == null ? "—" : `${Math.round(currentValue)}%`}
        </span>
        <span
          className={cn(
            "text-right font-mono text-muted-foreground tabular-nums",
            expanded ? "text-sm" : "text-[11px]",
          )}
        >
          {row.detail ?? ""}
        </span>
        <Sparkline
          values={row.history}
          color={row.color}
          showBaseline={!expanded}
          className={cn(
            "w-full overflow-visible",
            // Stretch to the row so the trend, not the whitespace, fills the
            // mini monitor.
            expanded ? "h-full min-h-16 self-stretch" : "h-9",
          )}
        />
      </div>
    );
  },
);

/**
 * Compact view: a dense one-line-per-metric strip meant for a small window
 * kept in a screen corner. `fillViewport` is the mini-monitor mode, where the
 * strip is the only thing on screen and its rows share the full height.
 * Footer items follow the same rule as rows: they appear once their metric is
 * actually collected.
 */
export const CompactStrip = ({
  className,
  fillViewport = false,
}: {
  className?: string;
  fillViewport?: boolean;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const cpuHistory = useAtomValue(cpuUsageHistoryAtom);
  const memoryHistory = useAtomValue(memoryUsageHistoryAtom);
  const gpuUsageHistories = useAtomValue(gpuUsageHistoriesAtom);
  const cpuTemperatures = useAtomValue(cpuTempAtom);
  const gpuTemperatureMap = useAtomValue(gpuTempMapAtom);
  const selectedGpuId = useAtomValue(selectedGpuIdAtom);

  const effectiveGpuId = getEffectiveGpuId(
    selectedGpuId,
    gpuUsageHistories,
    gpuTemperatureMap,
  );

  const rows: CompactMetricRow[] = [
    {
      id: "cpu",
      label: t("pages.performance.metrics.cpu"),
      color: toCssColor(settings.lineGraphColor.cpu),
      history: cpuHistory,
      detail: formatTemperature(
        cpuTemperatures[0]?.value,
        settings.temperatureUnit,
      ),
    },
    {
      id: "memory",
      label: t("pages.performance.metrics.memory"),
      color: toCssColor(settings.lineGraphColor.memory),
      history: memoryHistory,
    },
    ...(effectiveGpuId != null
      ? [
          {
            id: "gpu",
            label: t("pages.performance.metrics.gpu"),
            color: toCssColor(settings.lineGraphColor.gpu),
            history: gpuUsageHistories[effectiveGpuId] ?? [],
            detail: formatTemperature(
              gpuTemperatureMap[effectiveGpuId]?.value,
              settings.temperatureUnit,
            ),
          },
        ]
      : []),
  ];

  // Disk activity, network throughput, and the process count belong on this
  // strip once the backend collects them; they join `rows` / `footerItems`
  // without further layout changes. Storage capacity deliberately stays out:
  // it is a specification fact, not a live reading (see ADR 0014).
  const footerItems: string[] = [];

  return (
    <section
      className={cn(
        fillViewport
          ? "flex h-full w-full flex-col px-2"
          : "mx-auto w-full max-w-3xl rounded-2xl bg-card px-4 py-1.5",
        className,
      )}
      aria-label={t("pages.performance.views.compact")}
      data-testid="performance-compact-strip"
    >
      {rows.map((row) => (
        <CompactRow key={row.id} row={row} expanded={fillViewport} />
      ))}
      {footerItems.length > 0 && (
        <div className="flex gap-4 border-border/60 border-t py-2 font-mono text-[11px] text-muted-foreground tabular-nums">
          {footerItems.map((item) => (
            <span key={item}>{item}</span>
          ))}
        </div>
      )}
    </section>
  );
};
