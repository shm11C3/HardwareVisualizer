import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { XIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { setCloseToTrayPreference } from "@/hooks/useCloseToTrayPreference";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

const EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED = "close-to-tray-choice-requested";

export const CloseToTrayFirstRunDialog = () => {
  const { t } = useTranslation();
  const { error } = useTauriDialog();
  const [open, setOpen] = useState(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: The listener is registered once for the app lifetime.
  useEffect(() => {
    let off: (() => void) | undefined;
    let isCancelled = false;

    const setupListener = async () => {
      try {
        const unlisten = await listen(
          EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED,
          () => {
            setOpen(true);
          },
        );

        if (isCancelled) {
          unlisten();
        } else {
          off = unlisten;
        }

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

  const continueInBackground = async () => {
    try {
      const saved = await setCloseToTrayPreference(true);

      if (!saved) {
        await error(t("closeToTray.errors.continueInBackground"));
        return;
      }

      await getCurrentWindow().hide();
      setOpen(false);
    } catch (err) {
      console.error("Failed to continue in background:", err);
      await error(t("closeToTray.errors.continueInBackground"));
    }
  };

  const quitApp = async () => {
    try {
      const saved = await setCloseToTrayPreference(false);

      if (!saved) {
        await error(t("closeToTray.errors.quitApp"));
        return;
      }

      setOpen(false);
      await commands.quitApp();
    } catch (err) {
      console.error("Failed to quit from close-to-tray dialog:", err);
      await error(t("closeToTray.errors.quitApp"));
    }
  };

  return (
    <AlertDialog open={open}>
      <AlertDialogContent className="text-foreground">
        <Button
          aria-label={t("closeToTray.firstRunDialog.quitApp")}
          className="absolute top-4 right-4 h-8 w-8 border-0 bg-transparent p-0 opacity-70 hover:bg-muted hover:opacity-100"
          onClick={quitApp}
          type="button"
          variant="outline"
        >
          <XIcon className="size-4" />
          <span className="sr-only">
            {t("closeToTray.firstRunDialog.quitApp")}
          </span>
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
          <AlertDialogCancel onClick={quitApp}>
            {t("closeToTray.firstRunDialog.quitApp")}
          </AlertDialogCancel>
          <AlertDialogAction onClick={continueInBackground}>
            {t("closeToTray.firstRunDialog.continueInBackground")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};
