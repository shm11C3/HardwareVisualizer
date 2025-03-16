import { commands } from "@/rspc/bindings";
import type { Dispatch, SetStateAction } from "react";
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
} from "../ui/alert-dialog";

export const NeedRestart = ({
  alertOpen,
  setAlertOpen,
}: { alertOpen: boolean; setAlertOpen: Dispatch<SetStateAction<boolean>> }) => {
  const { t } = useTranslation();

  return (
    <AlertDialog open={alertOpen}>
      <AlertDialogContent className="dark:text-white">
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t("pages.settings.insights.needRestart.title")}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t("pages.settings.insights.needRestart.description")}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={() => setAlertOpen(false)}>
            {t("pages.settings.insights.needRestart.cancel")}
          </AlertDialogCancel>
          <AlertDialogAction onClick={commands.restartApp}>
            {t("pages.settings.insights.needRestart.restart")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};
