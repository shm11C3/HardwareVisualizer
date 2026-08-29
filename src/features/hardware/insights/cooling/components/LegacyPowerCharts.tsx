import { ChevronLeft, ChevronRight, ZapIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { InsightChart } from "@/features/hardware/insights/components/InsightChart";
import type { ArchivePeriod } from "@/features/hardware/insights/utils/archivePeriod";
import type { DataStats } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { DataArchiveHardwareType } from "@/rspc/bindings";
import type { CoolingPeriodRoute } from "../utils/coolingPeriodRoute";

/**
 * One power chart, driven by the Cooling tab's single period selector.
 * Offset paging (the chevrons) is kept per-panel, matching the pre-#2018
 * behavior of scrubbing through history one bucket window at a time.
 */
const PowerChart = ({
  type,
  stats,
  minutes,
}: {
  type: DataArchiveHardwareType;
  stats: DataStats;
  minutes: ArchivePeriod;
}) => {
  const { t } = useTranslation();
  const [offset, setOffset] = useState(0);
  const [intervalId, setIntervalId] = useState<ReturnType<
    typeof setInterval
  > | null>(null);

  const powerTitles: Partial<Record<DataArchiveHardwareType, string>> = {
    cpuPower: t("pages.performance.power.cpu"),
    gpuPower: t("pages.performance.power.gpu"),
    anePower: t("pages.performance.power.ane"),
    packagePower: t("pages.performance.power.package"),
  };
  const powerTitle = powerTitles[type];
  const title = powerTitle ? `${powerTitle} (W)` : type;

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
    <div className="overflow-hidden rounded-2xl bg-card p-4">
      <h3 className="flex items-center gap-1 py-1 font-bold text-lg">
        <ZapIcon className="shrink-0" size={18} />
        {title} ({t(`shared.${stats}`)})
      </h3>
      <div className="flex items-center justify-between">
        <button
          type="button"
          className="h-40 cursor-pointer text-muted-foreground disabled:pointer-events-none disabled:opacity-50"
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
          period={minutes}
          dataStats={stats}
          offset={offset}
        />
        <button
          type="button"
          className="h-40 cursor-pointer text-muted-foreground disabled:pointer-events-none disabled:opacity-50"
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
    </div>
  );
};

/**
 * The power archive charts the Cooling tab carried before the decided
 * layout. The temperature charts they used to sit beside are now the
 * thermal timeline lane; these stay, unchanged in behavior, until the
 * decided layout's power lane lands (#2021) - dropping them first would
 * lose working functionality for the sake of a half-built replacement.
 *
 * There is no daily rollup for power, so 90d/1y renders nothing here.
 */
export const LegacyPowerCharts = ({ route }: { route: CoolingPeriodRoute }) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();

  if (route.kind !== "archive" || settings.powerDisplayTargets.length === 0) {
    return null;
  }

  const charts = settings.powerDisplayTargets.flatMap(
    (target): { type: DataArchiveHardwareType; stats: DataStats }[] => {
      const type = `${target}Power` as DataArchiveHardwareType;
      return [
        { type, stats: "avg" },
        { type, stats: "max" },
        { type, stats: "min" },
      ];
    },
  );

  return (
    <section className="space-y-2" data-testid="cooling-legacy-power-charts">
      <h3 className="font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
        {t("pages.insights.cooling.legacyPowerCharts.title")}
      </h3>
      <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
        {charts.map((chart) => (
          <PowerChart
            key={`${chart.type}-${chart.stats}`}
            type={chart.type}
            stats={chart.stats}
            minutes={route.minutes}
          />
        ))}
      </div>
    </section>
  );
};
