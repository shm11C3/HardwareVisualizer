import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { DatabaseConversionStateBody } from "@/features/settings/components/insights/DatabaseConversionStateBody";
import { useDatabaseConversion } from "@/features/settings/hooks/useDatabaseConversion";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useTauriStore } from "@/hooks/useTauriStore";

const DISMISSED_STORE_KEY = "databaseConversionPromptDismissed";

/**
 * App-root, one-time prompt for the #2136 native database conversion:
 * users who already have Insights recording on must see this right after
 * updating, without opening the Insights screen first - the Settings entry
 * point (`DatabaseConversionSettings`) alone isn't enough for that. Mounted
 * beside `NavigationRestructureNotice` and `CloseToTrayFirstRunDialog`,
 * following the same app-wide-one-time-dialog shape as the latter
 * (`AlertDialog`, a controlled `open` boolean, no `onOpenChange`).
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
}: {
  settingsLoaded?: boolean;
}) => {
  const { t } = useTranslation();
  const conversion = useDatabaseConversion();
  const { settings } = useSettingsAtom();
  const [dismissed, setDismissed, dismissedPending] = useTauriStore(
    DISMISSED_STORE_KEY,
    false,
  );
  const [open, setOpen] = useState(false);
  const [everOffered, setEverOffered] = useState(false);

  const eligible =
    settingsLoaded &&
    !dismissedPending &&
    !dismissed &&
    settings.hardwareArchive.enabled &&
    (conversion.state.kind === "sqliteAuthoritative" ||
      conversion.state.kind === "conversionRecoverable");

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

        <DatabaseConversionStateBody {...conversion} onCompleted={close} />

        {(isInitialState || isActionRequired) && (
          <AlertDialogFooter>
            {isInitialState && (
              <AlertDialogCancel onClick={dismissForGood}>
                {t("databaseConversionPrompt.later")}
              </AlertDialogCancel>
            )}
            {isActionRequired && (
              <Button type="button" variant="outline" onClick={close}>
                {t("databaseConversionPrompt.close")}
              </Button>
            )}
          </AlertDialogFooter>
        )}
      </AlertDialogContent>
    </AlertDialog>
  );
};
