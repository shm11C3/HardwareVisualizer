import { ThermometerIcon } from "@phosphor-icons/react";
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
 * One provisional chart panel: the existing CPU-temperature/power archive
 * chart (`InsightChart`), restyled into the new panel chrome and driven by
 * the Cooling tab's single period selector instead of its own dropdown.
 * Offset paging (the chevrons) is kept per-panel, matching the pre-#2018
 * behavior of scrubbing through history one bucket window at a time.
 */
const ProvisionalChart = ({
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
  const title =
    type === "cpuTemperature"
      ? `CPU ${t("shared.temperature.full")}`
      : powerTitle
        ? `${powerTitle} (W)`
        : type;

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
        {type === "cpuTemperature" ? (
          <ThermometerIcon className="shrink-0" />
        ) : (
          <ZapIcon className="shrink-0" size={18} />
        )}
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
 * Zone (2) of the Cooling Insight layout. At 24h/7d/30d it keeps the
 * existing CPU-temperature and power archive charts working (provisional
 * until #2019 replaces them with the real thermal timeline). At 90d/1y
 * there is no archive-bucket equivalent, so it shows a placeholder instead
 * of fabricating a chart ahead of #2019.
 */
export const ThermalTimelineLane = ({
  route,
}: {
  route: CoolingPeriodRoute;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();

  if (route.kind === "dailyTrend") {
    return (
      <section
        className="rounded-2xl bg-card p-4"
        data-testid="cooling-thermal-timeline-lane"
      >
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.timeline.dailyPlaceholder")}
        </p>
      </section>
    );
  }

  const charts: { type: DataArchiveHardwareType; stats: DataStats }[] = [
    { type: "cpuTemperature", stats: "avg" },
    { type: "cpuTemperature", stats: "max" },
    { type: "cpuTemperature", stats: "min" },
    ...settings.powerDisplayTargets.flatMap(
      (target): { type: DataArchiveHardwareType; stats: DataStats }[] => {
        const type = `${target}Power` as DataArchiveHardwareType;
        return [
          { type, stats: "avg" },
          { type, stats: "max" },
          { type, stats: "min" },
        ];
      },
    ),
  ];

  return (
    <section
      className="grid grid-cols-1 gap-4 xl:grid-cols-2"
      data-testid="cooling-thermal-timeline-lane"
    >
      {charts.map((chart) => (
        <ProvisionalChart
          key={`${chart.type}-${chart.stats}`}
          type={chart.type}
          stats={chart.stats}
          minutes={route.minutes}
        />
      ))}
    </section>
  );
};
