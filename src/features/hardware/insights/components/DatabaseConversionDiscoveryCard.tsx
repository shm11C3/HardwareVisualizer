import { useSetAtom } from "jotai";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  DEFAULT_DISPLAY_TARGET,
  displayTargetAtom,
} from "@/features/menu/hooks/useMenu";
import { useDatabaseConversion } from "@/features/settings/hooks/useDatabaseConversion";
import { useTauriStore } from "@/hooks/useTauriStore";
import type { SelectedDisplayType } from "@/types/ui";

const DISCOVERY_CARD_DISMISSED_STORE_KEY =
  "databaseConversionDiscoveryCardDismissed";

/**
 * One-time discovery card for the #2136 native database conversion,
 * shown on the Insights page - the feature page users actually visit -
 * rather than requiring them to find it under Settings. Reuses
 * `useDatabaseConversion` (the same hook the Settings entry point uses)
 * so there is no second source of truth for the lifecycle state; this
 * page never needs the hook's own polling, since the card is hidden the
 * moment a conversion starts (`state.kind !== "sqliteAuthoritative"` and
 * `!== "conversionRecoverable"`).
 *
 * Dismissed = not shown again, tracked separately from the
 * conversion-complete notice's own "shown" flag (`useTauriStore`,
 * UI-local state, not an Application Preference).
 */
export const DatabaseConversionDiscoveryCard = () => {
  const { t } = useTranslation();
  const { state } = useDatabaseConversion();
  const setDisplayTargetAtom = useSetAtom(displayTargetAtom);
  const [, setStoredDisplayTarget] = useTauriStore<SelectedDisplayType>(
    "display",
    DEFAULT_DISPLAY_TARGET,
  );
  const [dismissed, setDismissed, dismissedPending] = useTauriStore(
    DISCOVERY_CARD_DISMISSED_STORE_KEY,
    false,
  );

  const shouldShow =
    !dismissedPending &&
    !dismissed &&
    (state.kind === "sqliteAuthoritative" ||
      state.kind === "conversionRecoverable");

  if (!shouldShow) {
    return null;
  }

  const openDatabaseConversionSettings = () => {
    setDisplayTargetAtom("settings");
    void setStoredDisplayTarget("settings");
  };

  return (
    <section
      className="mb-6 rounded-2xl bg-card p-4"
      data-testid="database-conversion-discovery-card"
      aria-label={t("pages.insights.databaseConversionDiscovery.title")}
    >
      <h3 className="font-semibold">
        {t("pages.insights.databaseConversionDiscovery.title")}
      </h3>
      <p className="mt-1 text-muted-foreground text-sm">
        {t("pages.insights.databaseConversionDiscovery.description")}
      </p>
      <div className="mt-3 flex gap-2">
        <Button
          type="button"
          size="sm"
          onClick={openDatabaseConversionSettings}
        >
          {t("pages.insights.databaseConversionDiscovery.action")}
        </Button>
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={() => void setDismissed(true)}
        >
          {t("pages.insights.databaseConversionDiscovery.later")}
        </Button>
      </div>
    </section>
  );
};
