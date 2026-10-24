import { listen } from "@tauri-apps/api/event";
import { XIcon } from "lucide-react";
import { useEffect, useLayoutEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { useTauriDialog } from "@/hooks/tauri/useTauriDialog";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

const EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED = "close-to-tray-choice-requested";
type CloseToTrayFirstRunDialogProps = {
  settingsLoaded?: boolean;
  deferred?: boolean;
  onRequestChange?: (requested: boolean) => void;
};

export const CloseToTrayFirstRunDialog = ({
  settingsLoaded = true,
  deferred = false,
  onRequestChange,
}: CloseToTrayFirstRunDialogProps) => {
  const { t } = useTranslation();
  const { error } = useTauriDialog();
  const { setCloseToTrayPreferenceAtom } = useSettingsAtom();
  const [requested, setRequested] = useState(false);
  const [saving, setSaving] = useState(false);

  useLayoutEffect(() => {
    onRequestChange?.(requested && settingsLoaded);
  }, [requested, settingsLoaded, onRequestChange]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: The listener is registered once for the app lifetime.
  useEffect(() => {
    let off: (() => void) | undefined;
    let isCancelled = false;

    const setupListener = async () => {
      try {
        const unlisten = await listen(
          EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED,
          () => {
            setRequested(true);
          },
        );

        if (isCancelled) {
          unlisten();
          return;
        }
        off = unlisten;

        const result = await commands.markCloseToTrayListenerReady();

        if (isError(result)) {
          console.error(
            "Failed to mark close-to-tray listener ready:",
            result.error,
          );
          await error(t("closeToTray.errors.listenerReady"));
        }
      } catch (err) {
        console.error("Failed to register close-to-tray dialog listener:", err);
        await error(t("closeToTray.errors.listenerReady"));
      }
    };

    setupListener();

    return () => {
      isCancelled = true;
      off?.();
    };
  }, []);

  const enableCloseToTray = async () => {
    setSaving(true);
    try {
      const saved = await setCloseToTrayPreferenceAtom(true);

      if (!saved) {
        // The prompt is a modal flow, so a failed save stays a dialog.
        await error(t("closeToTray.errors.savePreference"));
        return;
      }

      const hidden = await commands.hideMainWindowToTray();
      if (isError(hidden)) throw new Error(hidden.error);
      setRequested(false);
    } catch (err) {
      console.error("Failed to continue in background:", err);
      await error(t("closeToTray.errors.continueInBackground"));
    } finally {
      setSaving(false);
    }
  };

  const rejectCloseToTray = async () => {
    setSaving(true);
    try {
      if (!(await setCloseToTrayPreferenceAtom(false))) {
        await error(t("closeToTray.errors.savePreference"));
        return;
      }
      await commands.quitApp();
      setRequested(false);
    } catch (err) {
      console.error("Failed to quit from close-to-tray prompt:", err);
      await error(t("closeToTray.errors.quitApp"));
    } finally {
      setSaving(false);
    }
  };

  return (
    <AlertDialog open={requested && settingsLoaded && !deferred}>
      <AlertDialogContent className="text-foreground">
        <Button
          aria-label={t("closeToTray.firstRunDialog.quitApp")}
          className="absolute top-4 right-4 h-8 w-8 border-0 bg-transparent p-0 opacity-70 hover:bg-muted hover:opacity-100"
          disabled={saving}
          onClick={() => void rejectCloseToTray()}
          type="button"
          variant="outline"
        >
          <XIcon className="size-4" />
        </Button>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t("closeToTray.firstRunDialog.title")}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t("closeToTray.firstRunDialog.description")}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <Button
            disabled={saving}
            variant="outline"
            onClick={() => void rejectCloseToTray()}
          >
            {t("closeToTray.firstRunDialog.quitApp")}
          </Button>
          <Button disabled={saving} onClick={() => void enableCloseToTray()}>
            {t("closeToTray.firstRunDialog.continueInBackground")}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};
