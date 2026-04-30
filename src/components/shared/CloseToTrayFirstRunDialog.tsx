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
import { setCloseToTrayPreference } from "@/hooks/useCloseToTrayPreference";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { commands } from "@/rspc/bindings";

const EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED = "close-to-tray-choice-requested";

export const CloseToTrayFirstRunDialog = () => {
  const { t } = useTranslation();
  const { error } = useTauriDialog();
  const [open, setOpen] = useState(false);

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
      } catch (err) {
        console.error("Failed to register close-to-tray dialog listener:", err);
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
      await setCloseToTrayPreference(true);
      await getCurrentWindow().hide();
      setOpen(false);
    } catch (err) {
      console.error("Failed to continue in background:", err);
      await error(t("closeToTray.errors.continueInBackground"));
    }
  };

  const quitApp = async () => {
    try {
      await setCloseToTrayPreference(false);
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
        <AlertDialogCancel
          aria-label={t("closeToTray.firstRunDialog.close")}
          className="absolute top-4 right-4 h-8 w-8 border-0 bg-transparent p-0 opacity-70 hover:bg-muted hover:opacity-100"
          onClick={() => setOpen(false)}
        >
          <XIcon className="size-4" />
          <span className="sr-only">
            {t("closeToTray.firstRunDialog.close")}
          </span>
        </AlertDialogCancel>
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
