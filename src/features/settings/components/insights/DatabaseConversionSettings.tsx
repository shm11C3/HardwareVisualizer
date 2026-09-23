import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useDatabaseConversion } from "@/features/settings/hooks/useDatabaseConversion";
import { useTauriStore } from "@/hooks/useTauriStore";
import type { ConversionStep } from "@/rspc/bindings";
import { DatabaseConversionCompleteNotice } from "./DatabaseConversionCompleteNotice";

const NOTICE_SHOWN_STORE_KEY = "databaseConversionCompleteNoticeShown";

/**
 * Explicit entry point for the native DuckDB database conversion (#2136).
 * Renders nothing when this build does not include the feature
 * (`state.kind === "notSupported"`), so it disappears entirely instead of
 * showing a control nothing behind it can act on.
 */
export const DatabaseConversionSettings = () => {
  const { t } = useTranslation();
  const { state, error, start, cancel, justCompleted, acknowledgeCompletion } =
    useDatabaseConversion();
  const [noticeShown, setNoticeShown, noticeShownPending] = useTauriStore(
    NOTICE_SHOWN_STORE_KEY,
    false,
  );

  if (state.kind === "notSupported") {
    return null;
  }

  const dismissNotice = async () => {
    await setNoticeShown(true);
    acknowledgeCompletion();
  };

  const showNotice =
    state.kind === "nativeAuthoritative" &&
    justCompleted &&
    !noticeShownPending &&
    !noticeShown;

  // A literal per-case lookup, not a template key: the generated i18n key
  // union rejects a dynamically built string.
  const stepLabel = (step: ConversionStep) => {
    switch (step) {
      case "preflight":
        return t("pages.settings.insights.databaseConversion.step.preflight");
      case "buildingCandidate":
        return t(
          "pages.settings.insights.databaseConversion.step.buildingCandidate",
        );
      case "finalizing":
        return t("pages.settings.insights.databaseConversion.step.finalizing");
      case "pausingProducers":
        return t(
          "pages.settings.insights.databaseConversion.step.pausingProducers",
        );
      case "reconciling":
        return t("pages.settings.insights.databaseConversion.step.reconciling");
      case "selecting":
        return t("pages.settings.insights.databaseConversion.step.selecting");
      case "resumingProducers":
        return t(
          "pages.settings.insights.databaseConversion.step.resumingProducers",
        );
    }
  };

  return (
    <div className="py-4">
      <h4 className="font-bold text-xl">
        {t("pages.settings.insights.databaseConversion.title")}
      </h4>
      <p className="mt-2 whitespace-pre-wrap text-sm">
        {t("pages.settings.insights.databaseConversion.description")}
      </p>

      <div className="mt-3">
        {(state.kind === "sqliteAuthoritative" ||
          state.kind === "conversionRecoverable") && (
          <Button type="button" onClick={() => void start()}>
            {t("pages.settings.insights.databaseConversion.convert")}
          </Button>
        )}

        {state.kind === "converting" && (
          <div className="flex items-center gap-3">
            <p className="text-sm">
              {t("pages.settings.insights.databaseConversion.converting", {
                step: stepLabel(state.step),
              })}
            </p>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => void cancel()}
            >
              {t("pages.settings.insights.databaseConversion.cancel")}
            </Button>
          </div>
        )}

        {state.kind === "nativeAuthoritative" && (
          <p className="text-sm">
            {t("pages.settings.insights.databaseConversion.complete")}
          </p>
        )}

        {state.kind === "actionRequired" && (
          <div>
            <p className="text-destructive text-sm">
              {t(
                `pages.settings.insights.databaseConversion.actionRequired.${state.reason}`,
                {
                  defaultValue: t(
                    "pages.settings.insights.databaseConversion.actionRequired.default",
                  ),
                },
              )}
            </p>
            <details className="mt-2 text-muted-foreground text-xs">
              <summary className="cursor-pointer font-medium text-foreground">
                {t(
                  "pages.settings.insights.databaseConversion.actionRequired.details",
                )}
              </summary>
              <p className="mt-2 whitespace-pre-wrap">{state.diagnostic}</p>
            </details>
          </div>
        )}

        {error && <p className="mt-2 text-destructive text-sm">{error}</p>}
      </div>

      {showNotice && (
        <DatabaseConversionCompleteNotice
          onDismiss={() => void dismissNotice()}
        />
      )}
    </div>
  );
};
