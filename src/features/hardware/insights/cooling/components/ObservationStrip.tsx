import { useTranslation } from "react-i18next";
import type { CoolingBaselineDelta } from "@/rspc/bindings";
import { resolveBaselineLifecycle } from "../utils/baselineLifecycle";

/**
 * Zone (1) of the Cooling Insight layout. #2018 implements only the shell
 * and the establishing empty state; the idle-drift content itself is
 * #2019's.
 */
export const ObservationStrip = ({
  baselineDelta,
}: {
  baselineDelta: CoolingBaselineDelta | null;
}) => {
  const { t } = useTranslation();
  const lifecycle = resolveBaselineLifecycle(baselineDelta?.baseline ?? null);

  return (
    <section
      className="rounded-2xl bg-card p-4"
      data-testid="cooling-observation-strip"
    >
      {lifecycle.kind === "establishing" && (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.observationStrip.establishing", {
            qualifyingDays: lifecycle.qualifyingDays,
            requiredDays: lifecycle.requiredDays,
          })}
        </p>
      )}
      {lifecycle.kind === "ready" && (
        <p className="text-muted-foreground text-sm">
          {t("pages.insights.cooling.observationStrip.placeholder")}
        </p>
      )}
    </section>
  );
};
