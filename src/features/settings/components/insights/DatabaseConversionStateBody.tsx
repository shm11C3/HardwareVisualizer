import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useTauriStore } from "@/hooks/useTauriStore";
import type { ConversionStep, DatabaseConversionState } from "@/rspc/bindings";
import { DatabaseConversionCompleteNotice } from "./DatabaseConversionCompleteNotice";

const NOTICE_SHOWN_STORE_KEY = "databaseConversionCompleteNoticeShown";

export type DatabaseConversionStateBodyProps = {
  state: DatabaseConversionState;
  error: string | null;
  start: () => Promise<boolean>;
  cancel: () => Promise<boolean>;
  justCompleted: boolean;
  acknowledgeCompletion: () => void;
  /** Called after the completion notice is dismissed (either choice), so a
   * caller that only wants this content visible while the conversion is
   * unresolved (the #2136 prompt dialog) can close itself. The Settings
   * entry point ignores this - the section just keeps showing the plain
   * "converted" confirmation. */
  onCompleted?: () => void;
};

/**
 * The #2136 native database conversion's state-driven content: the
 * Convert/retry action, live progress with Cancel, the ActionRequired
 * message and collapsible technical details, and the one-time completion
 * notice. Takes `useDatabaseConversion()`'s return value as props rather
 * than calling the hook itself, so each caller (the Settings entry point,
 * the app-root prompt dialog) owns exactly one hook instance/polling loop
 * for its own mount lifetime, while sharing this same rendering logic
 * instead of forking it.
 *
 * Renders nothing when this build does not include the feature
 * (`state.kind === "notSupported"`).
 */
export const DatabaseConversionStateBody = ({
  state,
  error,
  start,
  cancel,
  justCompleted,
  acknowledgeCompletion,
  onCompleted,
}: DatabaseConversionStateBodyProps) => {
  const { t } = useTranslation();
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
    onCompleted?.();
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
    <>
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
              <p className="mt-2 max-h-32 overflow-y-auto whitespace-pre-wrap">
                {state.diagnostic}
              </p>
            </details>
            {/* `start_database_conversion` re-inspects on-disk authority
             * fresh rather than trusting this owner's own previous state
             * (see `run_conversion`'s own documentation), so retrying is
             * safe exactly for the two issues the driver itself produced
             * by failing or being cancelled mid-run - not for an
             * `Authority`/open/creation disagreement `inspect_authority`
             * refused to guess at, which a retry would refuse the same
             * way. */}
            {(state.reason === "conversionFailed" ||
              state.reason === "conversionCancelled") && (
              <Button
                type="button"
                className="mt-2"
                onClick={() => void start()}
              >
                {t("pages.settings.insights.databaseConversion.retry")}
              </Button>
            )}
          </div>
        )}

        {error && <p className="mt-2 text-destructive text-sm">{error}</p>}
      </div>

      {showNotice && (
        <DatabaseConversionCompleteNotice
          onDismiss={() => void dismissNotice()}
        />
      )}
    </>
  );
};
