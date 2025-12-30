import { useSetAtom } from "jotai";
import type { Dispatch, SetStateAction } from "react";
import { useTranslation } from "react-i18next";
import { commands } from "@/rspc/bindings";
import { settingAtoms } from "@/store/ui";
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
  description,
}: {
  alertOpen: boolean;
  setAlertOpen: Dispatch<SetStateAction<boolean>>;
  description?: string;
}) => {
  const { t } = useTranslation();
  const setIsRequiredRestart = useSetAtom(settingAtoms.isRequiredRestart);

  return (
    <AlertDialog open={alertOpen}>
      <AlertDialogContent className="text-foreground">
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t("pages.settings.insights.needRestart.title")}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {description ??
              t("pages.settings.insights.needRestart.description")}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel
            onClick={() => {
              setAlertOpen(false);
              setIsRequiredRestart(true);
            }}
          >
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
