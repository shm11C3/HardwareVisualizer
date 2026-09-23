import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { AlertDialogFooter } from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { useDatabaseConversionNoticeShown } from "@/features/settings/hooks/useDatabaseConversionNoticeShown";
import type { ConversionStep, DatabaseConversionState } from "@/rspc/bindings";
import { DatabaseConversionCompleteNotice } from "./DatabaseConversionCompleteNotice";

/**
 * Whether the completion notice is (or would be) visible for `state`,
 * given this session's own `justCompleted` observation and the shared
 * "already shown" flag. Exported as a pure function - not just computed
 * inline in `DatabaseConversionStateBody` - so a caller that renders its
 * own fallback exit when the notice is *not* showing (the #2136 prompt
 * dialog, once `state.kind` is `nativeAuthoritative`) can ask the exact
 * same question `DatabaseConversionStateBody` answers internally, rather
 * than guessing at or re-deriving the condition. */
export const showsDatabaseConversionNotice = (
  state: DatabaseConversionState,
  justCompleted: boolean,
  noticeShown: boolean,
  noticeShownPending: boolean,
): boolean =>
  state.kind === "nativeAuthoritative" &&
  justCompleted &&
  !noticeShownPending &&
  !noticeShown;

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
  /** `"footer"` places this state's own primary action (Convert/Retry/
   * Cancel) inside an `AlertDialogFooter`-styled row (right-aligned on
   * desktop, stacked full-width at a narrow viewport) instead of the
   * Settings entry point's original inline arrangement - the prompt
   * dialog uses this for every state, including `converting`, where
   * Cancel then sits alone in that row. Defaults to `"inline"`, the
   * Settings entry point's original layout. */
  layout?: "inline" | "footer";
  /** A secondary action (e.g. "Later"/"Close") placed alongside the
   * primary action in the same footer row when `layout === "footer"`.
   * `undefined` (the default, and every non-initial/non-ActionRequired
   * state even in footer layout) means no secondary action for this
   * state - the primary action, if any, still renders alone in its own
   * footer row. */
  footerSecondaryAction?: ReactNode;
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
  layout = "inline",
  footerSecondaryAction,
}: DatabaseConversionStateBodyProps) => {
  const { t } = useTranslation();
  const [noticeShown, setNoticeShown, noticeShownPending] =
    useDatabaseConversionNoticeShown();

  if (state.kind === "notSupported") {
    return null;
  }

  const dismissNotice = async () => {
    try {
      await setNoticeShown(true);
    } finally {
      // Acknowledge and let the caller close even if persisting "shown"
      // failed: the user already made their choice (Keep, or a confirmed
      // Set to 1 Year), and refusing to close over a storage write this
      // dialog cannot retry would leave no way out. The next successful
      // load simply re-shows the notice once, which is a lesser cost than
      // stranding the user here.
      acknowledgeCompletion();
      onCompleted?.();
    }
  };

  const showNotice = showsDatabaseConversionNotice(
    state,
    justCompleted,
    noticeShown,
    noticeShownPending,
  );

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

  // The primary action for this state, if any - computed once so it can be
  // placed either inline (Settings, the default) or alongside a caller's
  // own secondary action in one footer row (the prompt dialog).
  const primaryAction = (() => {
    if (
      state.kind === "sqliteAuthoritative" ||
      state.kind === "conversionRecoverable"
    ) {
      return (
        <Button type="button" onClick={() => void start()}>
          {t("pages.settings.insights.databaseConversion.convert")}
        </Button>
      );
    }
    if (state.kind === "converting") {
      return (
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => void cancel()}
        >
          {t("pages.settings.insights.databaseConversion.cancel")}
        </Button>
      );
    }
    if (
      state.kind === "actionRequired" &&
      // `start_database_conversion` re-inspects on-disk authority fresh
      // rather than trusting this owner's own previous state (see
      // `run_conversion`'s own documentation). For unreadable native
      // metadata, retry is safe because the driver does not rename or replace
      // the file: it continues only if a fresh inspection can establish a
      // supported state, and otherwise leaves the issue in ActionRequired.
      (state.reason === "conversionFailed" ||
        state.reason === "conversionCancelled" ||
        state.reason === "nativeMetadataUnreadable")
    ) {
      return (
        <Button type="button" onClick={() => void start()}>
          {t("pages.settings.insights.databaseConversion.retry")}
        </Button>
      );
    }
    return null;
  })();

  const footerLayout = layout === "footer";
  const footerRow = (primaryActionNode: ReactNode, sizeClassName = "") =>
    primaryActionNode || footerSecondaryAction ? (
      <AlertDialogFooter className={sizeClassName}>
        {footerSecondaryAction}
        {primaryActionNode}
      </AlertDialogFooter>
    ) : null;

  return (
    <>
      <div className="mt-3">
        {(state.kind === "sqliteAuthoritative" ||
          state.kind === "conversionRecoverable") &&
          (footerLayout ? footerRow(primaryAction) : primaryAction)}

        {state.kind === "converting" &&
          (footerLayout ? (
            <>
              <p className="text-sm">
                {t("pages.settings.insights.databaseConversion.converting", {
                  step: stepLabel(state.step),
                })}
              </p>
              {footerRow(primaryAction)}
            </>
          ) : (
            <div className="flex items-center gap-3">
              <p className="text-sm">
                {t("pages.settings.insights.databaseConversion.converting", {
                  step: stepLabel(state.step),
                })}
              </p>
              {primaryAction}
            </div>
          ))}

        {/* The notice carries its own "conversion complete" heading, so
            this line only covers the fallback where the notice will not
            show (already shown once, or this mount missed the completion). */}
        {state.kind === "nativeAuthoritative" && !showNotice && (
          <>
            <p className="text-sm">
              {t("pages.settings.insights.databaseConversion.complete")}
            </p>
            {footerLayout && footerRow(null)}
          </>
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
            {footerLayout
              ? footerRow(primaryAction, "mt-2")
              : primaryAction && <div className="mt-2">{primaryAction}</div>}
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
