import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { CoolingBandComparison, TemperatureUnit } from "@/rspc/bindings";
import { resolveBaselineLifecycle } from "../utils/baselineLifecycle";
import {
  buildAmbientAdjustedDumbbellRows,
  buildLoadBandDumbbellRows,
  type LoadBandDumbbellRow,
} from "../utils/loadBandDumbbell";
import { LoadBandDumbbellChart } from "./LoadBandDumbbellChart";

type EstablishedComparison = Extract<
  CoolingBandComparison,
  { status: "established" }
>;

/**
 * Zone (5): the load-band comparison dumbbell chart and its data-state
 * panel. Both read the same establishing/established lifecycle
 * `CoolingBandComparison` carries, so the "not enough data yet" state stays
 * a fact Core computed rather than a frontend guess.
 */
export const LoadBandComparisonPanel = ({
  bandComparison,
  hasError = false,
  powerUnsupported = false,
  fanUnsupported = false,
  ambientSources = [],
}: {
  bandComparison: CoolingBandComparison | null;
  hasError?: boolean;
  /**
   * Whether the routed period is known to carry no CPU package power
   * (#2021). The data-state row below names the sensors still missing, so
   * it may only name power on evidence - never while the answer is still
   * unknown, and never on a machine whose timeline draws a power lane.
   */
  powerUnsupported?: boolean;
  /** The same contract for the fan lane (#2022). */
  fanUnsupported?: boolean;
  /**
   * Sensor Source Labels the routed window's ambient archive actually
   * carried (#2046). Empty when the routed period has none - including on
   * the long-range routes, which read the daily rollup and so have no
   * ambient series to name a source from. The row is then simply absent
   * rather than claiming an unnamed source.
   */
  ambientSources?: readonly string[];
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
  // Null - not empty - on a machine with no environmental sensor, which is
  // what keeps this panel rendering exactly as it did before #2046 there.
  const ambientRows = useMemo(
    () =>
      bandComparison?.status === "established"
        ? buildAmbientAdjustedDumbbellRows(
            bandComparison.bands,
            temperatureUnit,
          )
        : null,
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
        {hasError && (
          <p className="text-muted-foreground text-sm">
            {t("pages.insights.cooling.loadBandComparison.loadFailed")}
          </p>
        )}
        {!hasError && lifecycle.kind === "loading" && <PanelLoadingSkeleton />}
        {lifecycle.kind === "establishing" && (
          <p className="text-muted-foreground text-sm">
            {t("pages.insights.cooling.dataState.establishing", {
              qualifyingDays: lifecycle.qualifyingDays,
              requiredDays: lifecycle.requiredDays,
            })}
          </p>
        )}
        {lifecycle.kind === "ready" &&
          (ambientRows == null || bandComparison?.status !== "established" ? (
            <LoadBandDumbbellChart
              rows={rows}
              temperatureUnit={temperatureUnit}
            />
          ) : (
            <ComparisonVariants
              bandComparison={bandComparison}
              rows={rows}
              ambientRows={ambientRows}
              temperatureUnit={temperatureUnit}
            />
          ))}
      </div>
      <div className="rounded-2xl bg-card p-4">
        <h3 className="mb-2 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
          {t("pages.insights.cooling.dataState.title")}
        </h3>
        {hasError && (
          <p className="text-muted-foreground text-sm">
            {t("pages.insights.cooling.loadBandComparison.loadFailed")}
          </p>
        )}
        {!hasError && lifecycle.kind === "loading" && <PanelLoadingSkeleton />}
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
            <DataStateDetails
              bandComparison={bandComparison}
              powerUnsupported={powerUnsupported}
              fanUnsupported={fanUnsupported}
              ambientSources={ambientSources}
            />
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

/**
 * The absolute comparison and its ambient-adjusted twin, stacked (#2046).
 *
 * Only mounted once ambient data exists, which is also the only reason the
 * comparison windows are labeled at all: with one chart the windows were
 * unambiguous, but the ΔT baseline establishes over its own days (see
 * `CoolingDeltaBaselineState`), so the two charts routinely compare
 * *different* baseline ranges against the same recent one. Leaving that
 * unlabeled would present two differently-scoped readings as one.
 */
const ComparisonVariants = ({
  bandComparison,
  rows,
  ambientRows,
  temperatureUnit,
}: {
  bandComparison: EstablishedComparison;
  rows: LoadBandDumbbellRow[];
  ambientRows: LoadBandDumbbellRow[];
  temperatureUnit: TemperatureUnit;
}) => {
  const { t } = useTranslation();
  const ambientBaseline = bandComparison.ambientAdjustedBaseline;

  return (
    <div className="space-y-4">
      <div className="space-y-2">
        <p className="font-medium text-xs">
          {t("pages.insights.cooling.loadBandComparison.absoluteTitle")}
        </p>
        <p className="text-muted-foreground text-xs">
          {t("pages.insights.cooling.loadBandComparison.window", {
            baselineStart: bandComparison.baselineWindowStartDate,
            baselineEnd: bandComparison.baselineWindowEndDate,
            recentStart: bandComparison.recentWindowStartDate,
            recentEnd: bandComparison.recentWindowEndDate,
          })}
        </p>
        <LoadBandDumbbellChart rows={rows} temperatureUnit={temperatureUnit} />
      </div>

      <div className="space-y-2 border-t pt-4">
        <p className="font-medium text-xs">
          {t("pages.insights.cooling.loadBandComparison.ambientAdjustedTitle")}
        </p>
        <p className="text-muted-foreground text-xs">
          {/* The ΔT baseline's own window, never the absolute one's - and
              its establishing progress when it has none yet, rather than a
              window borrowed from the chart above. */}
          {ambientBaseline.status === "established"
            ? t("pages.insights.cooling.loadBandComparison.window", {
                baselineStart: ambientBaseline.windowStartDate,
                baselineEnd: ambientBaseline.windowEndDate,
                recentStart: bandComparison.recentWindowStartDate,
                recentEnd: bandComparison.recentWindowEndDate,
              })
            : t("pages.insights.cooling.dataState.establishing", {
                qualifyingDays: ambientBaseline.qualifyingDays,
                requiredDays: ambientBaseline.requiredDays,
              })}
        </p>
        <LoadBandDumbbellChart
          rows={ambientRows}
          temperatureUnit={temperatureUnit}
          testId="cooling-load-band-dumbbell-ambient"
        />
      </div>
    </div>
  );
};

/** Summed paired minutes across every band, per comparison window. */
const ambientPairedMinutes = (bandComparison: EstablishedComparison) =>
  bandComparison.bands.reduce(
    (totals, entry) => ({
      baseline:
        totals.baseline + (entry.ambientAdjusted?.baseline.sampleMinutes ?? 0),
      recent:
        totals.recent + (entry.ambientAdjusted?.recent.sampleMinutes ?? 0),
    }),
    { baseline: 0, recent: 0 },
  );

const DataStateDetails = ({
  bandComparison,
  powerUnsupported,
  fanUnsupported,
  ambientSources,
}: {
  bandComparison: EstablishedComparison;
  powerUnsupported: boolean;
  fanUnsupported: boolean;
  ambientSources: readonly string[];
}) => {
  const { t } = useTranslation();
  // Only a band that carries an ambient reading licenses an ambient row:
  // on a machine with no environmental sensor every entry is null and the
  // panel keeps exactly the rows it had before #2046.
  const hasAmbientReading = bandComparison.bands.some(
    (entry) => entry.ambientAdjusted != null,
  );
  const ambientMinutes = ambientPairedMinutes(bandComparison);
  // The row names only the sensors actually still missing, and disappears
  // once neither is: a "Not supported yet" line beside a rendered lane
  // would contradict the timeline directly above it.
  const unsupportedLabelKey =
    powerUnsupported && fanUnsupported
      ? "pages.insights.cooling.dataState.unsupported.label"
      : powerUnsupported
        ? "pages.insights.cooling.dataState.unsupported.labelPowerOnly"
        : "pages.insights.cooling.dataState.unsupported.labelFanOnly";

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
      {ambientSources.length > 0 && (
        <div
          className="flex items-center justify-between gap-2"
          data-testid="cooling-data-state-ambient-source"
        >
          <dt className="text-muted-foreground">
            {t("pages.insights.cooling.dataState.ambient.sourceLabel")}
          </dt>
          {/* Named individually: the ambient archive is row-per-source, so
              which sensors contributed is a reading of its own. */}
          <dd>{ambientSources.join(", ")}</dd>
        </div>
      )}
      {hasAmbientReading && (
        <div
          className="flex items-center justify-between gap-2"
          data-testid="cooling-data-state-ambient-coverage"
        >
          <dt className="text-muted-foreground">
            {t("pages.insights.cooling.dataState.ambient.coverageLabel")}
          </dt>
          <dd className="font-mono tabular-nums">
            {t("pages.insights.cooling.dataState.sampleMinutes", {
              baseline: ambientMinutes.baseline,
              recent: ambientMinutes.recent,
            })}
          </dd>
        </div>
      )}
      {(powerUnsupported || fanUnsupported) && (
        <div className="flex items-center justify-between gap-2">
          <dt className="text-muted-foreground">{t(unsupportedLabelKey)}</dt>
          <dd>{t("pages.insights.cooling.dataState.unsupported.value")}</dd>
        </div>
      )}
    </dl>
  );
};
