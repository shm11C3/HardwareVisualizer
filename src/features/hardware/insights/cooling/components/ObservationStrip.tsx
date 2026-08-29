import { useTranslation } from "react-i18next";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Skeleton } from "@/components/ui/skeleton";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type {
  CoolingBaselineDelta,
  CoolingBaselineState,
  TemperatureUnit,
} from "@/rspc/bindings";
import { resolveBaselineLifecycle } from "../utils/baselineLifecycle";
import {
  daysInclusive,
  type ObservationDisplay,
  resolveObservationDisplay,
} from "../utils/observationDisplay";
import { formatSignedTemperatureDelta } from "../utils/temperatureUnit";

const TONE_DOT_CLASSES: Record<ObservationDisplay["tone"], string> = {
  muted: "bg-muted-foreground",
  positive: "bg-teal-500",
  mild: "bg-amber-500",
  large: "bg-destructive",
};

/**
 * Zone (1) of the Cooling Insight layout: the idle-drift observation. Maps
 * Core's `CoolingDeltaObservation` straight to display text and a status
 * dot color - it never re-derives what counts as a rise or how many days
 * make it "sustained" (see `resolveObservationDisplay`).
 */
export const ObservationStrip = ({
  baselineDelta,
  hasError = false,
}: {
  baselineDelta: CoolingBaselineDelta | null;
  hasError?: boolean;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const lifecycle = resolveBaselineLifecycle(baselineDelta?.baseline ?? null);

  return (
    <section
      className="space-y-2 rounded-2xl bg-card p-4"
      data-testid="cooling-observation-strip"
    >
      {hasError && (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.observationStrip.loadFailed")}
        </p>
      )}
      {!hasError && lifecycle.kind === "loading" && (
        <div aria-busy="true" data-testid="cooling-observation-strip-loading">
          <span className="sr-only">{t("shared.loading")}</span>
          <div className="flex items-center gap-2">
            <Skeleton className="h-2 w-2 rounded-full" />
            <Skeleton className="h-4 w-2/3" />
          </div>
          <Skeleton className="mt-2 h-3 w-1/3" />
        </div>
      )}
      {lifecycle.kind === "establishing" && (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.observationStrip.establishing", {
            qualifyingDays: lifecycle.qualifyingDays,
            requiredDays: lifecycle.requiredDays,
          })}
        </p>
      )}
      {lifecycle.kind === "ready" &&
        baselineDelta != null &&
        baselineDelta.baseline.status === "established" && (
          <EstablishedObservation
            baselineDelta={baselineDelta}
            baseline={baselineDelta.baseline}
            temperatureUnit={settings.temperatureUnit}
          />
        )}
    </section>
  );
};

const EstablishedObservation = ({
  baselineDelta,
  baseline,
  temperatureUnit,
}: {
  baselineDelta: CoolingBaselineDelta;
  baseline: Extract<CoolingBaselineState, { status: "established" }>;
  temperatureUnit: TemperatureUnit;
}) => {
  const { t } = useTranslation();
  const { observation, recent, delta, sustainedDays } = baselineDelta;
  const unitSuffix = temperatureUnit === "C" ? "°C" : "°F";

  if (observation === "establishing") {
    // Invariant: an established baseline never carries this observation
    // (see `derive_baseline_delta` in
    // `core/src/persistence/cooling_baseline_delta.rs`) - the parent
    // already gates on `resolveBaselineLifecycle`. Render nothing rather
    // than trust a state this component cannot make sense of.
    return null;
  }

  const display = resolveObservationDisplay(
    observation,
    delta,
    sustainedDays,
    temperatureUnit,
  );

  const label = (() => {
    switch (display.kind) {
      case "notComparable":
        return t("pages.insights.cooling.observationStrip.notComparable");
      case "withinRange":
        return t("pages.insights.cooling.observationStrip.withinRange", {
          delta: formatSignedTemperatureDelta(display.delta, unitSuffix),
        });
      case "sustainedMildRise":
        return t("pages.insights.cooling.observationStrip.sustainedMildRise", {
          delta: formatSignedTemperatureDelta(display.delta, unitSuffix),
          days: display.sustainedDays,
        });
      case "sustainedLargeRise":
        return t("pages.insights.cooling.observationStrip.sustainedLargeRise", {
          delta: formatSignedTemperatureDelta(display.delta, unitSuffix),
          days: display.sustainedDays,
        });
    }
  })();

  const showChecklist =
    display.kind === "sustainedMildRise" ||
    display.kind === "sustainedLargeRise";
  const showDisclaimer = display.kind !== "notComparable";

  return (
    <>
      <div className="flex items-start gap-2">
        <span
          aria-hidden
          className={`mt-1.5 h-2 w-2 shrink-0 rounded-full ${TONE_DOT_CLASSES[display.tone]}`}
        />
        <p className="text-sm">{label}</p>
      </div>

      {/* A comparison claim would contradict the not-comparable message
          right above it - the windows were NOT compared in that state. */}
      {display.kind !== "notComparable" && (
        <p className="text-muted-foreground text-xs">
          {t("pages.insights.cooling.observationStrip.comparisonWindow", {
            days: daysInclusive(recent.windowStartDate, recent.windowEndDate),
            startDate: baseline.windowStartDate,
            endDate: baseline.windowEndDate,
          })}
        </p>
      )}

      {showDisclaimer && (
        <p className="text-muted-foreground text-xs">
          {t("pages.insights.cooling.observationStrip.disclaimer")}
        </p>
      )}

      {showChecklist && (
        <Accordion type="single" collapsible>
          <AccordionItem value="checklist" className="border-none">
            <AccordionTrigger className="py-1 text-muted-foreground text-xs hover:no-underline [&>svg]:h-4 [&>svg]:w-4">
              {t("pages.insights.cooling.observationStrip.checklist.trigger")}
            </AccordionTrigger>
            <AccordionContent className="pb-1">
              <ul className="list-disc space-y-1 pl-4 text-muted-foreground text-xs">
                <li>
                  {t(
                    "pages.insights.cooling.observationStrip.checklist.items.dust",
                  )}
                </li>
                <li>
                  {t(
                    "pages.insights.cooling.observationStrip.checklist.items.airflow",
                  )}
                </li>
                <li>
                  {t(
                    "pages.insights.cooling.observationStrip.checklist.items.fanCurve",
                  )}
                </li>
                <li>
                  {t(
                    "pages.insights.cooling.observationStrip.checklist.items.thermalPaste",
                  )}
                </li>
              </ul>
              <p className="mt-2 text-muted-foreground text-xs">
                {t(
                  "pages.insights.cooling.observationStrip.checklist.footnote",
                )}
              </p>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      )}
    </>
  );
};
