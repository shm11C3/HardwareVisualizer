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
} from "@/components/ui/alert-dialog";
import { useMarkRestartRequired } from "@/hooks/settings/useRestartRequired";
import { commands } from "@/rspc/bindings";

export const NeedRestart = ({
  alertOpen,
  setAlertOpen,
  description,
  dismissible = true,
}: {
  alertOpen: boolean;
  setAlertOpen: Dispatch<SetStateAction<boolean>>;
  description?: string;
  dismissible?: boolean;
}) => {
  const { t } = useTranslation();
  const { markRestartRequired } = useMarkRestartRequired();

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
          {dismissible && (
            <AlertDialogCancel
              onClick={() => {
                setAlertOpen(false);
                markRestartRequired();
              }}
            >
              {t("pages.settings.insights.needRestart.cancel")}
            </AlertDialogCancel>
          )}
          <AlertDialogAction onClick={commands.restartApp}>
            {t("pages.settings.insights.needRestart.restart")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};
