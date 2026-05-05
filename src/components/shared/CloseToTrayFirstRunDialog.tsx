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
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

const EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED = "close-to-tray-choice-requested";
type PromptReason = "startup" | "close";

type CloseToTrayFirstRunDialogProps = {
  closeToTrayChoiceMade?: boolean;
  settingsLoaded?: boolean;
};

export const CloseToTrayFirstRunDialog = ({
  closeToTrayChoiceMade = false,
  settingsLoaded = true,
}: CloseToTrayFirstRunDialogProps) => {
  const { t } = useTranslation();
  const { error } = useTauriDialog();
  const { setCloseToTrayPreferenceAtom } = useSettingsAtom();
  const [promptReason, setPromptReason] = useState<PromptReason | null>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: The listener is registered once for the app lifetime.
  useEffect(() => {
    let off: (() => void) | undefined;
    let isCancelled = false;

    const setupListener = async () => {
      try {
        const unlisten = await listen(
          EVENT_CLOSE_TO_TRAY_CHOICE_REQUESTED,
          () => {
            setPromptReason("close");
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

  useEffect(() => {
    let isCancelled = false;

    const showStartupPromptIfNeeded = async () => {
      if (!settingsLoaded || closeToTrayChoiceMade) {
        return;
      }

      try {
        const availabilityResult = await commands.isCloseToTrayAvailable();

        if (isCancelled) {
          return;
        }

        if (isError(availabilityResult)) {
          console.error(
            "Failed to check close-to-tray availability for startup prompt:",
            availabilityResult.error,
          );
          return;
        }

        if (availabilityResult.data) {
          setPromptReason((currentReason) => currentReason ?? "startup");
        }
      } catch (err) {
        console.error(
          "Failed to initialize close-to-tray startup prompt:",
          err,
        );
      }
    };

    showStartupPromptIfNeeded();

    return () => {
      isCancelled = true;
    };
  }, [closeToTrayChoiceMade, settingsLoaded]);

  const enableCloseToTray = async () => {
    try {
      const saved = await setCloseToTrayPreferenceAtom(true);

      if (!saved) {
        return;
      }

      if (promptReason === "close") {
        await getCurrentWindow().hide();
      }

      setPromptReason(null);
    } catch (err) {
      console.error("Failed to continue in background:", err);
      await error(t("closeToTray.errors.continueInBackground"));
    }
  };

  const rejectCloseToTray = async () => {
    try {
      const saved = await setCloseToTrayPreferenceAtom(false);

      if (!saved) {
        return;
      }

      setPromptReason(null);

      if (promptReason === "close") {
        await commands.quitApp();
      }
    } catch (err) {
      console.error("Failed to reject close-to-tray prompt:", err);
      await error(
        t(
          promptReason === "close"
            ? "closeToTray.errors.quitApp"
            : "closeToTray.errors.savePreference",
        ),
      );
    }
  };

  const isStartupPrompt = promptReason === "startup";
  const dismissPrompt = () => {
    setPromptReason(null);
  };
  const titleKey = isStartupPrompt
    ? "closeToTray.startupDialog.title"
    : "closeToTray.firstRunDialog.title";
  const descriptionKey = isStartupPrompt
    ? "closeToTray.startupDialog.description"
    : "closeToTray.firstRunDialog.description";
  const acceptKey = isStartupPrompt
    ? "closeToTray.startupDialog.enable"
    : "closeToTray.firstRunDialog.continueInBackground";
  const rejectKey = isStartupPrompt
    ? "closeToTray.startupDialog.keepQuitOnClose"
    : "closeToTray.firstRunDialog.quitApp";
  const closeButtonKey = isStartupPrompt
    ? "closeToTray.firstRunDialog.close"
    : "closeToTray.firstRunDialog.quitApp";

  return (
    <AlertDialog open={promptReason !== null}>
      <AlertDialogContent className="text-foreground">
        <Button
          aria-label={t(closeButtonKey)}
          className="absolute top-4 right-4 h-8 w-8 border-0 bg-transparent p-0 opacity-70 hover:bg-muted hover:opacity-100"
          onClick={isStartupPrompt ? dismissPrompt : rejectCloseToTray}
          type="button"
          variant="outline"
        >
          <XIcon className="size-4" />
          <span className="sr-only">{t(closeButtonKey)}</span>
        </Button>
        <AlertDialogHeader>
          <AlertDialogTitle>{t(titleKey)}</AlertDialogTitle>
          <AlertDialogDescription>{t(descriptionKey)}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel onClick={rejectCloseToTray}>
            {t(rejectKey)}
          </AlertDialogCancel>
          <AlertDialogAction onClick={enableCloseToTray}>
            {t(acceptKey)}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};
