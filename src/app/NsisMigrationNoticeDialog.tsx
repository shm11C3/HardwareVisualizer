import { BundleType, getBundleType } from "@tauri-apps/api/app";
import { ChevronDownIcon, ExternalLinkIcon, EyeOffIcon } from "lucide-react";
import { useEffect, useState } from "react";
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
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { openURL } from "@/lib/openUrl";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const NSIS_MIGRATION_DOWNLOAD_URL = "https://hardviz.com/#download";

const MIGRATION_STEPS = ["step1", "step2", "step3", "step4"] as const;

type NsisMigrationNoticeDialogProps = {
  dismissed: boolean;
  settingsLoaded?: boolean;
  /** Another startup AlertDialog is open; wait so modals never stack. */
  deferred?: boolean;
};

/**
 * Recommends moving from the NSIS (.exe) build to the MSI (#2215).
 *
 * The NSIS build installs per user into a folder unelevated processes can
 * modify, which weakens the features that run the app as administrator
 * (#2216), and it cannot offer PawnIO setup at install time (ADR 0024). Only
 * the NSIS bundle shows this; dev and unbundled builds report no bundle type.
 */
export const NsisMigrationNoticeDialog = ({
  dismissed,
  settingsLoaded = true,
  deferred = false,
}: NsisMigrationNoticeDialogProps) => {
  const { t } = useTranslation();
  const { error } = useTauriDialog();
  const [isNsisBuild, setIsNsisBuild] = useState(false);
  // "Remind me next time" only hides the notice until the app restarts.
  const [hiddenThisSession, setHiddenThisSession] = useState(false);

  useEffect(() => {
    if (!settingsLoaded || dismissed) {
      return;
    }

    let isCancelled = false;
    getBundleType()
      .then((bundleType) => {
        if (!isCancelled) {
          setIsNsisBuild(bundleType === BundleType.Nsis);
        }
      })
      .catch((err) => {
        console.error("Failed to read the bundle type:", err);
      });

    return () => {
      isCancelled = true;
    };
  }, [settingsLoaded, dismissed]);

  if (
    !settingsLoaded ||
    dismissed ||
    hiddenThisSession ||
    !isNsisBuild ||
    deferred
  ) {
    return null;
  }

  const handleOpenDownload = async () => {
    try {
      await openURL(NSIS_MIGRATION_DOWNLOAD_URL);
    } catch (err) {
      console.error("Failed to open the download page:", err);
      await error(t("nsisMigrationNotice.errors.openDownload"));
    }
  };

  const handleNeverShowAgain = async () => {
    try {
      const result = await commands.dismissNsisMigrationNotice();
      if (isError(result)) {
        console.error("Failed to dismiss the migration notice:", result.error);
        await error(t("nsisMigrationNotice.errors.dismiss"));
        return;
      }
      setHiddenThisSession(true);
    } catch (err) {
      console.error("Failed to dismiss the migration notice:", err);
      await error(t("nsisMigrationNotice.errors.dismiss"));
    }
  };

  return (
    <AlertDialog open>
      {/* Scroll inside the dialog so the actions stay reachable in compact
          windows. */}
      <AlertDialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto text-foreground">
        <AlertDialogHeader>
          <AlertDialogTitle>{t("nsisMigrationNotice.title")}</AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3 text-left text-muted-foreground text-sm leading-relaxed">
              <p>{t("nsisMigrationNotice.why")}</p>
              <div className="space-y-1">
                <div className="font-medium text-foreground">
                  {t("nsisMigrationNotice.stepsTitle")}
                </div>
                <ol className="list-decimal space-y-1 pl-5">
                  {MIGRATION_STEPS.map((step) => (
                    <li key={step}>{t(`nsisMigrationNotice.steps.${step}`)}</li>
                  ))}
                </ol>
              </div>
              <p>{t("nsisMigrationNotice.data")}</p>
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter className="gap-2">
          <Button
            onClick={() => void handleOpenDownload()}
            type="button"
            variant="outline"
          >
            <ExternalLinkIcon className="size-4" />
            {t("nsisMigrationNotice.actions.openDownload")}
          </Button>
          <DropdownMenu modal={false}>
            <DropdownMenuTrigger asChild>
              <Button type="button" variant="secondary">
                <EyeOffIcon className="size-4" />
                {t("nsisMigrationNotice.actions.hide")}
                <ChevronDownIcon className="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={() => setHiddenThisSession(true)}>
                {t("nsisMigrationNotice.actions.remindLater")}
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => void handleNeverShowAgain()}>
                {t("nsisMigrationNotice.actions.neverShowAgain")}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};
