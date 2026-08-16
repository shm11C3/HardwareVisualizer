import { useAtomValue } from "jotai";
import type { CSSProperties } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import { processorsUsageHistoryAtom } from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { toCssColor } from "./InstrumentStrip";

/**
 * Current load per logical processor as plain bars. Bars stay cheap enough to
 * repaint at the 1Hz update rate, unlike one line chart per core.
 */
export const PerCorePanel = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const processorsUsageHistory = useAtomValue(processorsUsageHistoryAtom);
  const currentUsages = processorsUsageHistory.at(-1) ?? [];
  const color = toCssColor(settings.lineGraphColor.cpu);

  if (currentUsages.length === 0) {
    // An empty history means no hardware-monitor sample has arrived yet;
    // absence is only a fact once a sample exists without per-core data.
    return processorsUsageHistory.length > 0 ? (
      <p className="px-4 pb-4 text-muted-foreground text-sm">
        {t("pages.performance.perCoreUnavailable")}
      </p>
    ) : (
      <div className="px-4 pb-4">
        <Skeleton
          className="h-16 w-full rounded-md"
          data-testid="per-core-loading"
        />
      </div>
    );
  }

  return (
    // Column count follows the panel's own width, not the viewport: the panel
    // is half as wide in the two-column layout, where viewport breakpoints
    // would still ask for four unreadable columns.
    <div
      className="grid grid-cols-[repeat(auto-fill,minmax(8.5rem,1fr))] gap-x-6 gap-y-1.5 p-4 pt-2"
      style={{ "--metric-color": color } as CSSProperties}
    >
      {currentUsages.map((usage, index) => (
        <div
          // biome-ignore lint/suspicious/noArrayIndexKey: processor order is stable
          key={index}
          className="grid grid-cols-[2.4rem_1fr_2.8rem] items-center gap-2"
        >
          <span className="font-mono text-[11px] text-muted-foreground tabular-nums">
            P{index}
          </span>
          <span
            className="h-1.5 overflow-hidden rounded-full bg-muted"
            aria-hidden="true"
          >
            {/* scaleX keeps per-core updates compositor-only; width would
                rerun layout for core-count bars every tick. */}
            <span
              className="block h-full w-full origin-left rounded-full bg-[var(--metric-color)] transition-transform duration-300 ease-out motion-reduce:transition-none"
              style={{
                transform: `scaleX(${Math.min(100, Math.max(0, usage)) / 100})`,
              }}
            />
          </span>
          <span className="text-right font-mono text-xs tabular-nums">
            {Math.round(usage)}%
          </span>
        </div>
      ))}
    </div>
  );
};
