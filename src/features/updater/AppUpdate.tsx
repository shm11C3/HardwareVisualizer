import { useEffect, useState } from "react";
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
  const {
    meta,
    installing,
    percent,
    downloaded,
    total,
    install,
    isFinished,
    installError,
  } = useUpdater();
  const { t } = useTranslation();

  const errorMessage = installError
    ? installError.message === "NoPendingUpdate"
      ? t("pages.updater.noPendingUpdate")
      : t("pages.updater.installFailed", { message: installError.message })
    : null;

  if (installError?.kind === "restart-required") {
    return <RestartRequiredAfterUpdate message={installError.message} />;
  }

  if (errorMessage) {
    return (
      <AppUpdateModal
        meta={meta}
        install={install}
        errorMessage={errorMessage}
      />
    );
  }

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

  return <AppUpdateModal meta={meta} install={install} errorMessage={null} />;
}

function AppUpdateModal({
  meta,
  install,
  errorMessage,
}: {
  meta: ReturnType<typeof useUpdater>["meta"];
  install: ReturnType<typeof useUpdater>["install"];
  errorMessage: string | null;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  useEffect(() => {
    if (meta || errorMessage) {
      setOpen(true);
    }
  }, [meta, errorMessage]);

  return (
    <AlertDialog open={open} onOpenChange={setOpen}>
      <AlertDialogContent className="text-foreground">
        <AlertDialogHeader>
          <AlertDialogTitle className="text-foreground">
            {t("pages.updater.title")}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t("pages.updater.description", { version: meta?.version })}
          </AlertDialogDescription>
          {errorMessage && (
            <AlertDialogDescription role="alert">
              {errorMessage}
            </AlertDialogDescription>
          )}
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
        <div>
          <p className="text-neutral-700 text-sm dark:text-neutral-200">
            {t("pages.updater.currentVersion", {
              currentVersion: meta?.currentVersion,
            })}
          </p>
          <p className="text-neutral-700 text-sm dark:text-neutral-200">
            {t("pages.updater.newVersion", {
              newVersion: meta?.version,
            })}
          </p>
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={() => setOpen(false)}>
            {t("pages.updater.later")}
          </AlertDialogCancel>
          <AlertDialogAction
            disabled={!meta}
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

function RestartRequiredAfterUpdate({ message }: { message: string }) {
  const { t } = useTranslation();
  const [alertOpen, setAlertOpen] = useState(true);

  return (
    <NeedRestart
      alertOpen={alertOpen}
      setAlertOpen={setAlertOpen}
      dismissible={false}
      description={t("pages.updater.restartRequired", { message })}
    />
  );
}
