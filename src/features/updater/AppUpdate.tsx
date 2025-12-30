import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NeedRestart } from "@/components/shared/System";
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
import { UpdateTopBar } from "./components/UpdateBar";
import { useUpdater } from "./hooks/useAppUpdate";

export function AppUpdate() {
  const { meta, installing, percent, downloaded, total, install, isFinished } =
    useUpdater();

  if (installing && !isFinished && percent !== null) {
    return (
      <UpdateTopBar
        percent={percent}
        transferredBytes={downloaded}
        totalBytes={total}
      />
    );
  }

  if (isFinished) {
    return <RestartOnUpdateComplete />;
  }

  return <AppUpdateModal meta={meta} install={install} />;
}

function AppUpdateModal({
  meta,
  install,
}: {
  meta: ReturnType<typeof useUpdater>["meta"];
  install: ReturnType<typeof useUpdater>["install"];
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(!!meta);

  return (
    <AlertDialog open={open}>
      <AlertDialogContent className="text-foreground">
        <AlertDialogHeader>
          <AlertDialogTitle className="text-foreground">
            {t("pages.updater.title")}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t("pages.updater.description", { version: meta?.version })}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <p
          className="text-neutral-700 text-sm dark:text-neutral-200"
          style={{ wordBreak: "auto-phrase" }}
        >
          {t("pages.updater.releaseNotesDescription", {
            releaseNotesUrl:
              "https://github.com/shm11C3/HardwareVisualizer/releases/latest",
          })}
        </p>
        <p className="text-neutral-700 text-sm dark:text-neutral-200">
          {t("pages.updater.currentVersion", {
            currentVersion: meta?.currentVersion,
          })}
        </p>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={() => setOpen(false)}>
            {t("pages.updater.later")}
          </AlertDialogCancel>
          <AlertDialogAction
            onClick={() => {
              setOpen(false);
              install();
            }}
          >
            {t("pages.updater.updateAndRestart")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

function RestartOnUpdateComplete() {
  const { t } = useTranslation();
  const [alertOpen, setAlertOpen] = useState(true);

  return (
    <NeedRestart
      alertOpen={alertOpen}
      setAlertOpen={setAlertOpen}
      description={t("pages.updater.needRestart")}
    />
  );
}
