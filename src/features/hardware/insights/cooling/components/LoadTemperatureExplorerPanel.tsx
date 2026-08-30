import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { CoolingLoadTemperatureExplorer } from "@/rspc/bindings";
import { useCoolingLoadTemperatureExplorer } from "../hooks/useCoolingLoadTemperatureExplorer";
import { resolveBaselineLifecycle } from "../utils/baselineLifecycle";
import {
  buildExplorerBandDeltaRows,
  buildExplorerMedianTrend,
  buildExplorerMinimapSegments,
  buildExplorerScatterPoints,
  defaultExplorerRecentDays,
  type ExplorerRecentDays,
  explorerRecentDayPresets,
  isExplorerRecentDays,
} from "../utils/loadTemperatureExplorer";
import { formatSignedTemperatureDelta } from "../utils/temperatureUnit";
import { ExplorerWindowMinimap } from "./ExplorerWindowMinimap";
import { LoadTemperatureScatterChart } from "./LoadTemperatureScatterChart";

const ACCORDION_VALUE = "explorer";

/**
 * The load-vs-temperature Explorer (#2023): a secondary analysis that sits
 * below the load-band comparison, collapsed by default.
 *
 * Collapsed is not just a layout choice - the query only runs once the
 * panel is open (see `useCoolingLoadTemperatureExplorer`), so a folded
 * secondary view costs nothing. The expanded/collapsed state is UI-local
 * and intentionally not persisted: it is a transient reading posture, not
 * an Application Preference.
 */
export const LoadTemperatureExplorerPanel = () => {
  const { t } = useTranslation();
  const [expanded, setExpanded] = useState(false);
  const [recentDays, setRecentDays] = useState<ExplorerRecentDays>(
    defaultExplorerRecentDays,
  );
  const { data: explorer, hasError } = useCoolingLoadTemperatureExplorer(
    expanded ? recentDays : null,
  );

  return (
    <section
      className="rounded-2xl bg-card px-4"
      data-testid="cooling-explorer-panel"
    >
      <Accordion
        type="single"
        collapsible
        value={expanded ? ACCORDION_VALUE : ""}
        onValueChange={(value) => setExpanded(value === ACCORDION_VALUE)}
      >
        <AccordionItem value={ACCORDION_VALUE} className="border-none">
          <AccordionTrigger
            className="font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em] hover:no-underline"
            data-testid="cooling-explorer-trigger"
          >
            {t("pages.insights.cooling.explorer.title")}
          </AccordionTrigger>
          <AccordionContent className="space-y-4 pb-4">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <p className="text-muted-foreground text-xs">
                {t("pages.insights.cooling.explorer.description")}
              </p>
              <Select
                value={String(recentDays)}
                onValueChange={(next) => {
                  const parsed = Number.parseInt(next, 10);
                  if (isExplorerRecentDays(parsed)) {
                    setRecentDays(parsed);
                  }
                }}
              >
                <SelectTrigger
                  className="w-[160px]"
                  data-testid="cooling-explorer-window-select"
                >
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {explorerRecentDayPresets.map((days) => (
                    <SelectItem key={days} value={String(days)}>
                      {t("pages.insights.cooling.explorer.recentWindow", {
                        days,
                      })}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>

            <ExplorerBody explorer={explorer} hasError={hasError} />
          </AccordionContent>
        </AccordionItem>
      </Accordion>
    </section>
  );
};

/**
 * The four data states the Explorer can be in, kept distinct: a load
 * failure is not an empty period, and neither is a request still in
 * flight (DP-02).
 */
const ExplorerBody = ({
  explorer,
  hasError,
}: {
  explorer: CoolingLoadTemperatureExplorer | null;
  hasError: boolean;
}) => {
  const { t } = useTranslation();
  const lifecycle = resolveBaselineLifecycle(explorer);

  if (hasError) {
    return (
      <p className="text-muted-foreground text-sm">
        {t("pages.insights.cooling.explorer.loadFailed")}
      </p>
    );
  }

  if (lifecycle.kind === "loading") {
    return (
      <div aria-busy="true" data-testid="cooling-explorer-loading">
        <span className="sr-only">{t("shared.loading")}</span>
        <Skeleton className="h-72 w-full" />
      </div>
    );
  }

  if (lifecycle.kind === "establishing") {
    return (
      <p className="text-muted-foreground text-sm">
        {t("pages.insights.cooling.dataState.establishing", {
          qualifyingDays: lifecycle.qualifyingDays,
          requiredDays: lifecycle.requiredDays,
        })}
      </p>
    );
  }

  if (explorer?.status !== "established") {
    return null;
  }

  return <EstablishedExplorer explorer={explorer} />;
};

const EstablishedExplorer = ({
  explorer,
}: {
  explorer: Extract<CoolingLoadTemperatureExplorer, { status: "established" }>;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const temperatureUnit = settings.temperatureUnit;
  const unitSuffix = temperatureUnit === "C" ? "°C" : "°F";

  const baselinePoints = useMemo(
    () => buildExplorerScatterPoints(explorer.baseline, temperatureUnit),
    [explorer.baseline, temperatureUnit],
  );
  const recentPoints = useMemo(
    () => buildExplorerScatterPoints(explorer.recent, temperatureUnit),
    [explorer.recent, temperatureUnit],
  );
  const baselineMedians = useMemo(
    () =>
      buildExplorerMedianTrend(
        explorer.bandDeltas,
        "baseline",
        temperatureUnit,
      ),
    [explorer.bandDeltas, temperatureUnit],
  );
  const recentMedians = useMemo(
    () =>
      buildExplorerMedianTrend(explorer.bandDeltas, "recent", temperatureUnit),
    [explorer.bandDeltas, temperatureUnit],
  );
  const deltaRows = useMemo(
    () => buildExplorerBandDeltaRows(explorer.bandDeltas, temperatureUnit),
    [explorer.bandDeltas, temperatureUnit],
  );
  const minimapSegments = useMemo(
    () => buildExplorerMinimapSegments(explorer.baseline, explorer.recent),
    [explorer.baseline, explorer.recent],
  );

  return (
    <div className="space-y-4">
      <ExplorerWindowMinimap segments={minimapSegments} />

      <LoadTemperatureScatterChart
        baselinePoints={baselinePoints}
        recentPoints={recentPoints}
        baselineMedians={baselineMedians}
        recentMedians={recentMedians}
        temperatureUnit={temperatureUnit}
      />

      <dl className="space-y-1.5 text-xs" data-testid="cooling-explorer-deltas">
        {deltaRows.map((row) => (
          <div
            key={row.band}
            className="flex items-center justify-between gap-2"
          >
            <dt className="text-muted-foreground">
              {t(`pages.insights.cooling.loadBands.${row.band}`)}
            </dt>
            <dd className="flex items-center gap-3">
              {row.comparable ? (
                <>
                  <span className="font-mono text-muted-foreground tabular-nums">
                    {row.baseline.toFixed(1)}
                    {unitSuffix} → {row.recent.toFixed(1)}
                    {unitSuffix}
                  </span>
                  <span className="font-mono tabular-nums">
                    {formatSignedTemperatureDelta(row.delta, unitSuffix)}
                  </span>
                </>
              ) : (
                <span className="text-muted-foreground italic">
                  {t("pages.insights.cooling.explorer.notComparable", {
                    baselineHours: row.baselinePointCount,
                    recentHours: row.recentPointCount,
                  })}
                </span>
              )}
            </dd>
          </div>
        ))}
      </dl>

      <p className="text-muted-foreground text-xs">
        {t("pages.insights.cooling.explorer.footnote")}
      </p>
    </div>
  );
};
