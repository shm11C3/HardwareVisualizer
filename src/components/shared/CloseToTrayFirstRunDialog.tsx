import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
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
import { commands } from "@/rspc/bindings";

const EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED = "close-to-tray-choice-requested";

export const CloseToTrayFirstRunDialog = () => {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  useEffect(() => {
    const unlisten = listen(EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED, () => {
      setOpen(true);
    });

    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const continueInBackground = async () => {
    await setCloseToTrayPreference(true);
    setOpen(false);
    await getCurrentWindow().hide();
  };

  const quitApp = async () => {
    await setCloseToTrayPreference(false);
    setOpen(false);
    await commands.quitApp();
  };

  return (
    <AlertDialog open={open}>
      <AlertDialogContent className="text-foreground">
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
