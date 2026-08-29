import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { CoolingBandComparison } from "@/rspc/bindings";
import { resolveBaselineLifecycle } from "../utils/baselineLifecycle";
import { buildLoadBandDumbbellRows } from "../utils/loadBandDumbbell";
import { LoadBandDumbbellChart } from "./LoadBandDumbbellChart";

/**
 * Zone (5): the load-band comparison dumbbell chart and its data-state
 * panel. Both read the same establishing/established lifecycle
 * `CoolingBandComparison` carries, so the "not enough data yet" state stays
 * a fact Core computed rather than a frontend guess.
 */
export const LoadBandComparisonPanel = ({
  bandComparison,
}: {
  bandComparison: CoolingBandComparison | null;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const temperatureUnit = settings.temperatureUnit;
  const lifecycle = resolveBaselineLifecycle(bandComparison);

  const rows = useMemo(
    () =>
      bandComparison?.status === "established"
        ? buildLoadBandDumbbellRows(bandComparison.bands, temperatureUnit)
        : [],
    [bandComparison, temperatureUnit],
  );

  return (
    <section
      className="grid grid-cols-1 gap-4 xl:grid-cols-2"
      data-testid="cooling-load-band-panel"
    >
      <div className="rounded-2xl bg-card p-4">
        <h3 className="mb-2 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
          {t("pages.insights.cooling.loadBandComparison.title")}
        </h3>
        {lifecycle.kind === "loading" && <PanelLoadingSkeleton />}
        {lifecycle.kind === "establishing" && (
          <p className="text-muted-foreground text-sm">
            {t("pages.insights.cooling.dataState.establishing", {
              qualifyingDays: lifecycle.qualifyingDays,
              requiredDays: lifecycle.requiredDays,
            })}
          </p>
        )}
        {lifecycle.kind === "ready" && (
          <LoadBandDumbbellChart
            rows={rows}
            temperatureUnit={temperatureUnit}
          />
        )}
      </div>
      <div className="rounded-2xl bg-card p-4">
        <h3 className="mb-2 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
          {t("pages.insights.cooling.dataState.title")}
        </h3>
        {lifecycle.kind === "loading" && <PanelLoadingSkeleton />}
        {lifecycle.kind === "establishing" && (
          <p className="text-muted-foreground text-sm">
            {t("pages.insights.cooling.dataState.establishing", {
              qualifyingDays: lifecycle.qualifyingDays,
              requiredDays: lifecycle.requiredDays,
            })}
          </p>
        )}
        {lifecycle.kind === "ready" &&
          bandComparison?.status === "established" && (
            <DataStateDetails bandComparison={bandComparison} />
          )}
      </div>
    </section>
  );
};

/**
 * Shared loading placeholder for both boxes below: `bandComparison` has not
 * resolved yet (`resolveBaselineLifecycle(null)`), which is distinct from
 * Core's own "establishing" fact - render a modest, i18n-labeled skeleton
 * instead of leaving the box blank while the request is in flight.
 */
const PanelLoadingSkeleton = () => {
  const { t } = useTranslation();

  return (
    <div aria-busy="true" data-testid="cooling-load-band-panel-loading">
      <span className="sr-only">{t("shared.loading")}</span>
      <div className="space-y-1.5">
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-5/6" />
        <Skeleton className="h-3 w-2/3" />
      </div>
    </div>
  );
};

const DataStateDetails = ({
  bandComparison,
}: {
  bandComparison: Extract<CoolingBandComparison, { status: "established" }>;
}) => {
  const { t } = useTranslation();

  return (
    <dl className="space-y-1.5 text-xs">
      {bandComparison.bands.map((entry) => (
        <div
          key={entry.band}
          className="flex items-center justify-between gap-2"
        >
          <dt className="text-muted-foreground">
            {t(`pages.insights.cooling.loadBands.${entry.band}`)}
          </dt>
          <dd className="font-mono tabular-nums">
            {t("pages.insights.cooling.dataState.sampleMinutes", {
              baseline: entry.baseline.sampleMinutes,
              recent: entry.recent.sampleMinutes,
            })}
          </dd>
        </div>
      ))}
      <div className="flex items-center justify-between gap-2 border-t pt-1.5">
        <dt className="text-muted-foreground">
          {t("pages.insights.cooling.dataState.temperatureSource.label")}
        </dt>
        <dd>{t("pages.insights.cooling.dataState.temperatureSource.value")}</dd>
      </div>
      <div className="flex items-center justify-between gap-2">
        <dt className="text-muted-foreground">
          {t("pages.insights.cooling.dataState.unsupported.label")}
        </dt>
        <dd>{t("pages.insights.cooling.dataState.unsupported.value")}</dd>
      </div>
    </dl>
  );
};
