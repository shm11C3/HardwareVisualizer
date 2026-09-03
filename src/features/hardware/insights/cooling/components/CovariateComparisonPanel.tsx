import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type {
  CoolingCovariateComparability,
  CoolingLoadBand,
  TemperatureUnit,
} from "@/rspc/bindings";
import { useCoolingCovariateComparison } from "../hooks/useCoolingCovariateComparison";
import type { AmbientCapability } from "../utils/ambientTimeline";
import { resolveBaselineLifecycle } from "../utils/baselineLifecycle";
import {
  buildCovariateLead,
  buildCovariateRows,
  buildFitLineChart,
  type CovariateRow,
  type CovariateTag,
  type EstablishedCovariateComparison,
  temperatureUnitSuffix,
} from "../utils/covariateComparison";
import { CovariateFitChart } from "./CovariateFitChart";

/**
 * The band the observation strip compares under: the idle drift is the
 * reading this panel explains, so the co-variates are read in the same
 * band.
 */
const COMPARED_BAND: CoolingLoadBand = "idle";

const TAG_CLASSES: Record<CovariateTag, string> = {
  moved: "border-amber-500/60 text-amber-500",
  withinRange: "border-border text-muted-foreground",
  notComparable: "border-border text-muted-foreground",
  notArchived: "border-border text-muted-foreground",
  removedByDelta: "border-border text-muted-foreground",
  atMatchedPower: "border-border text-muted-foreground",
};

const NO_VALUE = "—";

const NOT_COMPARABLE_KEYS: Record<
  CoolingCovariateComparability,
  | "pages.insights.cooling.covariateComparison.notComparable.tooFewPairedMinutes"
  | "pages.insights.cooling.covariateComparison.notComparable.differentAmbientSource"
  | null
> = {
  comparable: null,
  tooFewPairedMinutes:
    "pages.insights.cooling.covariateComparison.notComparable.tooFewPairedMinutes",
  differentAmbientSource:
    "pages.insights.cooling.covariateComparison.notComparable.differentAmbientSource",
};

/**
 * "What moved with it" (#2068): the archived co-variates of the Thermal
 * Delta - package power, each fan, the band's share of the day, ambient -
 * beside the Thermal Delta itself, across the same two windows the
 * ambient-adjusted observation compares. Every judgement, window, and fit
 * shown here is Core's; this panel formats them and says nothing Core did
 * not.
 *
 * Capability-dependent like the ambient lane: a machine the routed window
 * proves has no ambient source gets no panel. The no-ambient fallback the
 * issue describes - the factors it does have against absolute CPU
 * temperature - is out of scope here and deliberately not approximated.
 */
export const CovariateComparisonPanel = ({
  ambientCapability,
}: {
  ambientCapability: AmbientCapability;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { data, hasError } = useCoolingCovariateComparison(
    ambientCapability === "absent" ? null : COMPARED_BAND,
  );

  if (ambientCapability === "absent") {
    return null;
  }
  // The same gate as the strip's ambient-adjusted line (see
  // `resolveAmbientAdjustedDisplay`): a machine with no environmental
  // sensor reports an establishing Thermal Delta Baseline at zero
  // qualifying days, and that is the absence of evidence for a sensor, not
  // a sensor warming up (DP-02). It is what keeps the long-range routes -
  // whose capability is always `unknown` - from announcing an
  // establishing comparison on a machine that can never establish one.
  if (data?.status === "establishing" && data.qualifyingDays === 0) {
    return null;
  }

  const lifecycle = resolveBaselineLifecycle(data);

  return (
    <section
      className="space-y-3 rounded-2xl bg-card p-4"
      data-testid="cooling-covariate-panel"
    >
      <h3 className="font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
        {t("pages.insights.cooling.covariateComparison.title")}
      </h3>
      {hasError && (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.covariateComparison.loadFailed")}
        </p>
      )}
      {!hasError && lifecycle.kind === "loading" && (
        <div aria-busy="true" data-testid="cooling-covariate-panel-loading">
          <span className="sr-only">{t("shared.loading")}</span>
          <div className="space-y-1.5">
            <Skeleton className="h-3 w-full" />
            <Skeleton className="h-3 w-5/6" />
            <Skeleton className="h-3 w-2/3" />
          </div>
        </div>
      )}
      {lifecycle.kind === "establishing" && (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.dataState.establishing", {
            qualifyingDays: lifecycle.qualifyingDays,
            requiredDays: lifecycle.requiredDays,
          })}
        </p>
      )}
      {lifecycle.kind === "ready" && data?.status === "established" && (
        <EstablishedComparison
          comparison={data}
          temperatureUnit={settings.temperatureUnit}
        />
      )}
    </section>
  );
};

const EstablishedComparison = ({
  comparison,
  temperatureUnit,
}: {
  comparison: EstablishedCovariateComparison;
  temperatureUnit: TemperatureUnit;
}) => {
  const { t } = useTranslation();
  const bandLabel = t(`pages.insights.cooling.loadBands.${comparison.band}`);
  const rows = useMemo(
    () => buildCovariateRows(comparison, temperatureUnit),
    [comparison, temperatureUnit],
  );
  const chart = useMemo(
    () =>
      comparison.comparable
        ? buildFitLineChart(comparison, temperatureUnit)
        : null,
    [comparison, temperatureUnit],
  );
  // Core's contract is that `comparability` names a reason whenever
  // `comparable` is false; a response that contradicts it gets no reason
  // line rather than an invented one.
  const notComparableKey = NOT_COMPARABLE_KEYS[comparison.comparability];

  return (
    <>
      <p className="text-muted-foreground text-xs">
        {t("pages.insights.cooling.covariateComparison.subtitle", {
          band: bandLabel,
          baselineStart: comparison.baselineWindowStartDate,
          baselineEnd: comparison.baselineWindowEndDate,
          recentStart: comparison.recentWindowStartDate,
          recentEnd: comparison.recentWindowEndDate,
          baseline: comparison.baselinePairedMinutes,
          recent: comparison.recentPairedMinutes,
        })}
      </p>

      {comparison.comparable ? (
        <LeadSentence
          comparison={comparison}
          rows={rows}
          temperatureUnit={temperatureUnit}
        />
      ) : (
        notComparableKey != null && (
          <p
            className="text-muted-foreground text-sm"
            data-testid="cooling-covariate-not-comparable"
          >
            {t(notComparableKey, {
              baseline: comparison.baselinePairedMinutes,
              recent: comparison.recentPairedMinutes,
            })}
          </p>
        )
      )}

      <div className="grid grid-cols-1 gap-4 xl:grid-cols-2">
        <div className="space-y-3">
          <FactorTable rows={rows} bandLabel={bandLabel} />
          <p className="text-muted-foreground text-xs">
            {t("pages.insights.cooling.covariateComparison.footnote")}
          </p>
        </div>
        {/* The fits are only drawn against each other once Core says the
            windows compare: two lines from two ambient sources would be
            two sensors, not two windows. */}
        {chart != null && (
          <div className="space-y-2">
            <p className="font-medium text-xs">
              {t("pages.insights.cooling.covariateComparison.chart.title", {
                band: bandLabel,
                unit: temperatureUnitSuffix(temperatureUnit),
              })}
            </p>
            <CovariateFitChart
              chart={chart}
              temperatureUnit={temperatureUnit}
            />
            <p className="text-muted-foreground text-xs">
              {t("pages.insights.cooling.covariateComparison.chart.caption", {
                baseline: comparison.baselinePairedMinutes,
                recent: comparison.recentPairedMinutes,
              })}
            </p>
          </div>
        )}
      </div>
    </>
  );
};

/**
 * The one-sentence reading, assembled from i18n fragments: the ΔT change at
 * matched power, then what moved, then what stayed within range. Each
 * clause is absent rather than filled when Core produced nothing for it.
 */
const LeadSentence = ({
  comparison,
  rows,
  temperatureUnit,
}: {
  comparison: EstablishedCovariateComparison;
  rows: readonly CovariateRow[];
  temperatureUnit: TemperatureUnit;
}) => {
  const { t, i18n } = useTranslation();
  const lead = useMemo(
    () => buildCovariateLead(comparison, rows, temperatureUnit),
    [comparison, rows, temperatureUnit],
  );
  const factorName = useFactorName();
  const listFormat = useMemo(
    () =>
      new Intl.ListFormat(i18n?.language || undefined, {
        type: "conjunction",
      }),
    [i18n?.language],
  );
  const listOf = (items: readonly CovariateRow[], withChange: boolean) =>
    listFormat.format(
      items.map((row) =>
        withChange && row.change != null
          ? t(
              "pages.insights.cooling.covariateComparison.lead.factorWithChange",
              {
                factor: factorName(row),
                change: row.change,
              },
            )
          : factorName(row),
      ),
    );

  const clauses = [
    lead.deltaAtMatchedPower == null
      ? null
      : t(
          "pages.insights.cooling.covariateComparison.lead.deltaAtMatchedPower",
          {
            delta: lead.deltaAtMatchedPower,
          },
        ),
    lead.moved.length === 0
      ? null
      : t("pages.insights.cooling.covariateComparison.lead.moved", {
          factors: listOf(lead.moved, true),
        }),
    lead.withinRange.length === 0
      ? null
      : t("pages.insights.cooling.covariateComparison.lead.withinRange", {
          factors: listOf(lead.withinRange, false),
        }),
  ].filter((clause): clause is string => clause != null);

  if (clauses.length === 0) {
    return null;
  }

  return (
    <p className="text-sm" data-testid="cooling-covariate-lead">
      {clauses.join(
        t("pages.insights.cooling.covariateComparison.lead.sentenceSeparator"),
      )}
    </p>
  );
};

/** The localized name of a row's factor, with the band or fan it names. */
const useFactorName = () => {
  const { t } = useTranslation();
  return (row: CovariateRow, bandLabel?: string) => {
    switch (row.kind) {
      case "fan":
        return t("pages.insights.cooling.covariateComparison.factors.fan", {
          fanSource: row.fanSource,
        });
      case "thermalDelta":
      case "loadBandShare":
        return t(
          `pages.insights.cooling.covariateComparison.factors.${row.kind}`,
          {
            band:
              bandLabel ??
              t(`pages.insights.cooling.loadBands.${COMPARED_BAND}`),
          },
        );
      default:
        return t(
          `pages.insights.cooling.covariateComparison.factors.${row.kind}`,
        );
    }
  };
};

const FactorTable = ({
  rows,
  bandLabel,
}: {
  rows: readonly CovariateRow[];
  bandLabel: string;
}) => {
  const { t } = useTranslation();
  const factorName = useFactorName();

  return (
    <div className="overflow-x-auto">
      <table className="w-full text-xs" data-testid="cooling-covariate-table">
        <thead>
          <tr className="text-left text-muted-foreground">
            <th className="pb-1.5 font-normal">
              {t("pages.insights.cooling.covariateComparison.columns.factor")}
            </th>
            <th className="pb-1.5 text-right font-normal">
              {t("pages.insights.cooling.covariateComparison.columns.baseline")}
            </th>
            <th className="pb-1.5 text-right font-normal">
              {t("pages.insights.cooling.covariateComparison.columns.recent")}
            </th>
            <th className="pb-1.5 text-right font-normal">
              {t("pages.insights.cooling.covariateComparison.columns.change")}
            </th>
            <th className="pb-1.5">
              <span className="sr-only">
                {t("pages.insights.cooling.covariateComparison.columns.tag")}
              </span>
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr
              key={row.key}
              className="border-t"
              data-testid={`cooling-covariate-row-${row.kind}`}
            >
              <td className="py-1.5 pr-2">
                <span className="flex items-center gap-2">
                  <span
                    aria-hidden
                    className="h-2 w-2 shrink-0 rounded-full"
                    style={{ backgroundColor: row.color }}
                  />
                  {factorName(row, bandLabel)}
                </span>
              </td>
              <td className="py-1.5 pl-2 text-right font-mono tabular-nums">
                {row.baseline ?? NO_VALUE}
              </td>
              <td className="py-1.5 pl-2 text-right font-mono tabular-nums">
                {row.recent ?? NO_VALUE}
              </td>
              <td className="py-1.5 pl-2 text-right font-mono tabular-nums">
                {row.change ?? NO_VALUE}
              </td>
              <td className="py-1.5 pl-3">
                <span
                  className={`inline-block whitespace-nowrap rounded-full border px-2 py-0.5 text-[10px] leading-4 ${TAG_CLASSES[row.tag]}`}
                >
                  {t(
                    `pages.insights.cooling.covariateComparison.tags.${row.tag}`,
                  )}
                </span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};
