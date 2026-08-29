import { useTranslation } from "react-i18next";
import type { CoolingBandComparison } from "@/rspc/bindings";
import { resolveBaselineLifecycle } from "../utils/baselineLifecycle";

/**
 * Zone (5): the load-band comparison and its data-state panel. #2018
 * implements only the shell - the comparison bars themselves are #2020's.
 * The right-hand panel reads the same establishing/established lifecycle
 * `CoolingBandComparison` carries, so the "not enough data yet" state stays
 * a fact Core computed rather than a frontend guess.
 */
export const LoadBandComparisonPanel = ({
  bandComparison,
}: {
  bandComparison: CoolingBandComparison | null;
}) => {
  const { t } = useTranslation();
  const lifecycle = resolveBaselineLifecycle(bandComparison);

  return (
    <section
      className="grid grid-cols-1 gap-4 xl:grid-cols-2"
      data-testid="cooling-load-band-panel"
    >
      <div className="rounded-2xl bg-card p-4">
        <h3 className="mb-2 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
          {t("pages.insights.cooling.loadBandComparison.title")}
        </h3>
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.loadBandComparison.placeholder")}
        </p>
      </div>
      <div className="rounded-2xl bg-card p-4">
        <h3 className="mb-2 font-semibold text-muted-foreground text-xs uppercase tracking-[0.18em]">
          {t("pages.insights.cooling.dataState.title")}
        </h3>
        {lifecycle.kind === "establishing" && (
          <p className="text-muted-foreground text-sm">
            {t("pages.insights.cooling.dataState.establishing", {
              qualifyingDays: lifecycle.qualifyingDays,
              requiredDays: lifecycle.requiredDays,
            })}
          </p>
        )}
        {lifecycle.kind === "ready" && (
          <p className="text-muted-foreground text-sm">
            {t("pages.insights.cooling.dataState.ready")}
          </p>
        )}
      </div>
    </section>
  );
};
