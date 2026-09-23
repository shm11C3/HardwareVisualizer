import { useEffect, useLayoutEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
  DatabaseConversionStateBody,
  showsDatabaseConversionNotice,
} from "@/features/settings/components/insights/DatabaseConversionStateBody";
import { useDatabaseConversion } from "@/features/settings/hooks/useDatabaseConversion";
import { useDatabaseConversionNoticeShown } from "@/features/settings/hooks/useDatabaseConversionNoticeShown";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useTauriStore } from "@/hooks/useTauriStore";

const DISMISSED_STORE_KEY = "databaseConversionPromptDismissed";

type DatabaseConversionPromptDialogProps = {
  settingsLoaded?: boolean;
  /** Another startup AlertDialog is open; wait before opening. */
  deferred?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** Whether it is still unknown if this prompt will open on this launch. */
  onPendingChange?: (pending: boolean) => void;
};

/**
 * App-root, one-time prompt for the #2136 native database conversion:
 * users who already have Insights recording on must see this right after
 * updating, without opening the Insights screen first - the Settings entry
 * point (`DatabaseConversionSettings`) alone isn't enough for that. Mounted
 * beside `NavigationRestructureNotice` and `CloseToTrayFirstRunDialog`,
 * following the same app-wide-one-time-dialog shape as the latter
 * (`AlertDialog`, a controlled `open` boolean, no Radix `onOpenChange`, so
 * only its own buttons close it).
 *
 * `deferred` holds back the first open while another startup AlertDialog
 * is open, so modals never stack; App decides which ones count. It only
 * gates the opening: once shown, a conversion in progress stays visible
 * even if another dialog opens later. `onPendingChange` reports true until
 * the conversion state and the dismissal flag are known, so a dialog that
 * yields to this one can wait instead of opening first and being replaced.
 *
 * Shown only when the conversion is supported, the lifecycle state is
 * `sqliteAuthoritative` or `conversionRecoverable`, Insights recording
 * (`hardwareArchive.enabled`) is on, and the user has not already dismissed
 * it. A fresh install starts native-authoritative, so it never qualifies.
 * "Later" persists the dismissal for good (`databaseConversionPromptDismissed`,
 * UI-local Tauri Store state, not an Application Preference) - a cancelled
 * or failed attempt does not, so the prompt can still remind the user on a
 * later launch unless they explicitly said Later.
 *
 * Renders the same in-place flow the Settings section does
 * (`DatabaseConversionStateBody`, driven by this component's own
 * `useDatabaseConversion` instance) rather than sending the user away to
 * Settings: Convert now, live progress with Cancel, the one-time retention
 * notice on completion, and ActionRequired with retry and technical
 * details - no forked state logic.
 */
export const DatabaseConversionPromptDialog = ({
  settingsLoaded = true,
  deferred = false,
  onOpenChange,
  onPendingChange,
}: DatabaseConversionPromptDialogProps) => {
  const { t } = useTranslation();
  const conversion = useDatabaseConversion();
  const { settings } = useSettingsAtom();
  const [dismissed, setDismissed, dismissedPending] = useTauriStore(
    DISMISSED_STORE_KEY,
    false,
  );
  const [noticeShown, , noticeShownPending] =
    useDatabaseConversionNoticeShown();
  const [open, setOpen] = useState(false);
  const [everOffered, setEverOffered] = useState(false);

  // Before paint, so App defers other dialogs in the same frame.
  useLayoutEffect(() => {
    onOpenChange?.(open);
  }, [open, onOpenChange]);

  const wouldOpen =
    settingsLoaded &&
    !dismissedPending &&
    !dismissed &&
    settings.hardwareArchive.enabled &&
    (conversion.state.kind === "sqliteAuthoritative" ||
      conversion.state.kind === "conversionRecoverable");
  const eligible = wouldOpen && !deferred;

  // Pending until the state and the dismissal flag are known, and also while
  // this prompt is going to open but has not yet (the open lands in a passive
  // effect, so a dialog that yields to this one must not slip in before it).
  const pending =
    !conversion.settled ||
    dismissedPending ||
    Boolean(wouldOpen && !everOffered);
  useLayoutEffect(() => {
    onPendingChange?.(pending);
  }, [pending, onPendingChange]);

  useEffect(() => {
    if (!everOffered && eligible) {
      setOpen(true);
      setEverOffered(true);
    }
  }, [everOffered, eligible]);

  const dismissForGood = () => {
    void setDismissed(true);
    setOpen(false);
  };

  const close = () => setOpen(false);

  const isInitialState =
    conversion.state.kind === "sqliteAuthoritative" ||
    conversion.state.kind === "conversionRecoverable";
  const isActionRequired = conversion.state.kind === "actionRequired";
  // `nativeAuthoritative`'s only other exit is the completion notice's own
  // two buttons - but the notice does not render every time this state is
  // reached: `justCompleted` may already be false (this dialog observed
  // the transition in an earlier render this session and already
  // acknowledged it, or never observed it at all - e.g. the state was
  // already `nativeAuthoritative` the first time this mount polled), or
  // the "already shown" flag may already be `true` from a previous
  // session. Without a fallback here, a user who reaches this state any
  // way other than watching the notice appear would have no button at
  // all - `useDatabaseConversion`'s own `start()` now also primes its
  // completion detection for a conversion that finishes before this
  // dialog observes an intermediate `converting` poll, but that only
  // narrows the gap, it doesn't close every path into this state without
  // `justCompleted`. `showsDatabaseConversionNotice` is the exact
  // question `DatabaseConversionStateBody` answers internally to decide
  // whether to render the notice; asking it here rather than re-deriving
  // the condition keeps the two in agreement.
  const isNativeWithoutNotice =
    conversion.state.kind === "nativeAuthoritative" &&
    !showsDatabaseConversionNotice(
      conversion.state,
      conversion.justCompleted,
      noticeShown,
      noticeShownPending,
    );

  // Paired with this state's own primary action (Convert/Retry/Cancel) in
  // one footer row by `DatabaseConversionStateBody` - see its own
  // `footerSecondaryAction` documentation. `converting` intentionally has
  // none: Cancel sits alone in its footer row. Completion normally has
  // none either - the completion notice's own two buttons are its own
  // row - except the `isNativeWithoutNotice` fallback above.
  const footerSecondaryAction = isInitialState ? (
    <AlertDialogCancel onClick={dismissForGood}>
      {t("databaseConversionPrompt.later")}
    </AlertDialogCancel>
  ) : isActionRequired ? (
    <Button type="button" variant="outline" onClick={close}>
      {t("databaseConversionPrompt.close")}
    </Button>
  ) : isNativeWithoutNotice ? (
    <Button type="button" variant="outline" onClick={close}>
      {t("databaseConversionPrompt.done")}
    </Button>
  ) : undefined;

  return (
    <AlertDialog open={open}>
      <AlertDialogContent className="max-h-[85vh] overflow-y-auto text-foreground">
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t("databaseConversionPrompt.title")}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t("databaseConversionPrompt.description")}
          </AlertDialogDescription>
        </AlertDialogHeader>

        <DatabaseConversionStateBody
          {...conversion}
          onCompleted={close}
          layout="footer"
          footerSecondaryAction={footerSecondaryAction}
        />
      </AlertDialogContent>
    </AlertDialog>
  );
};
