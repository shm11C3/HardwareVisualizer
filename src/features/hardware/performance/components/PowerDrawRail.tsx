import { LightningIcon } from "@phosphor-icons/react";
import { useAtomValue } from "jotai";
import { useTranslation } from "react-i18next";
import { powerDrawAtom } from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import type { PowerDisplayTarget } from "@/rspc/bindings";

// Keep the same canonical order as the existing PowerPanel and target
// settings. The package total still gets an accent, but moving it ahead of
// CPU/GPU would make the two Power Draw surfaces disagree.
const railOrder: readonly PowerDisplayTarget[] = [
  "cpu",
  "gpu",
  "ane",
  "package",
];

const powerKey = {
  cpu: "cpuWatts",
  gpu: "gpuWatts",
  ane: "aneWatts",
  package: "packageWatts",
} as const;

export const PowerDrawRail = () => {
  const { t } = useTranslation();
  const power = useAtomValue(powerDrawAtom);
  const { settings } = useSettingsAtom();
  const targets = railOrder.filter((target) =>
    settings.powerDisplayTargets.includes(target),
  );

  return (
    <section
      className="flex shrink-0 flex-wrap items-center gap-x-5 gap-y-2 rounded-xl bg-card px-4 py-2.5"
      aria-label={t("pages.performance.panels.power")}
      data-testid="performance-monitor-power-rail"
    >
      <div className="flex items-center gap-2 text-muted-foreground">
        <LightningIcon size={18} className="text-amber-400" />
        <h3 className="font-mono font-semibold text-[11px] uppercase tracking-[0.18em]">
          {t("pages.performance.panels.power")}
        </h3>
      </div>
      <div className="grid min-w-0 flex-1 grid-cols-[repeat(auto-fit,minmax(6.5rem,1fr))] gap-x-5 gap-y-1">
        {targets.map((target) => {
          const watts = power[powerKey[target]];
          return (
            <div
              key={target}
              className={cn(
                "flex items-baseline justify-between gap-3 border-border/60 border-l pl-3 text-sm",
                target === "package" && "text-amber-400",
              )}
            >
              <span className="text-muted-foreground text-xs">
                {t(`pages.performance.power.${target}`)}
              </span>
              <strong className="font-mono font-semibold tabular-nums">
                {watts != null ? `${watts.toFixed(1)} W` : "—"}
              </strong>
            </div>
          );
        })}
      </div>
    </section>
  );
};
