import { ChevronDownIcon, ExternalLinkIcon, EyeOffIcon } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
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
import type { ExternalComponentGuidanceCandidate } from "@/rspc/bindings";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import type { SelectedDisplayType } from "@/types/ui";
import {
  externalComponentGuidanceCopyKey,
  externalComponentGuidanceDocsUrl,
  externalComponentGuidanceViewForDisplayTarget,
} from "./externalComponentGuidance";

type ExternalComponentGuidanceDialogProps = {
  displayTarget: SelectedDisplayType | null;
  settingsLoaded?: boolean;
};

export const ExternalComponentGuidanceDialog = ({
  displayTarget,
  settingsLoaded = true,
}: ExternalComponentGuidanceDialogProps) => {
  const { t, i18n } = useTranslation();
  const { error } = useTauriDialog();
  const [candidates, setCandidates] = useState<
    ExternalComponentGuidanceCandidate[]
  >([]);

  const view = useMemo(
    () => externalComponentGuidanceViewForDisplayTarget(displayTarget),
    [displayTarget],
  );

  useEffect(() => {
    if (!settingsLoaded || !view) {
      setCandidates([]);
      return;
    }

    let isCancelled = false;

    const loadCandidates = async () => {
      try {
        const result =
          await commands.getExternalComponentGuidanceCandidates(view);
        if (isCancelled) return;

        if (isError(result)) {
          console.error(
            "Failed to fetch external component guidance:",
            result.error,
          );
          return;
        }

        setCandidates(result.data);
      } catch (err) {
        if (isCancelled) return;
        console.error("Failed to fetch external component guidance:", err);
      }
    };

    void loadCandidates();
    const intervalId = window.setInterval(loadCandidates, 60_000);

    return () => {
      isCancelled = true;
      window.clearInterval(intervalId);
    };
  }, [settingsLoaded, view]);

  const candidate = candidates[0] ?? null;
  const copyKey = candidate
    ? externalComponentGuidanceCopyKey(candidate)
    : null;

  const removeCandidate = (key: string) => {
    setCandidates((current) => current.filter((item) => item.key !== key));
  };

  const handleNeverShowAgain = async () => {
    if (!candidate) return;

    try {
      const result = await commands.acknowledgeExternalComponentGuidanceKey(
        candidate.key,
      );
      if (isError(result)) {
        console.error(
          "Failed to acknowledge external component guidance:",
          result.error,
        );
        await error(t("externalComponentGuidance.errors.acknowledge"));
        return;
      }

      removeCandidate(candidate.key);
    } catch (err) {
      console.error("Failed to acknowledge external component guidance:", err);
      await error(t("externalComponentGuidance.errors.acknowledge"));
    }
  };

  const handleLater = async () => {
    if (!candidate) return;

    try {
      const result = await commands.deferExternalComponentGuidanceForSession(
        candidate.key,
      );
      if (isError(result)) {
        console.error(
          "Failed to defer external component guidance:",
          result.error,
        );
        await error(t("externalComponentGuidance.errors.defer"));
        return;
      }

      removeCandidate(candidate.key);
    } catch (err) {
      console.error("Failed to defer external component guidance:", err);
      await error(t("externalComponentGuidance.errors.defer"));
    }
  };

  const handleOpenDetails = async () => {
    if (!candidate) return;

    try {
      await openURL(
        externalComponentGuidanceDocsUrl(
          candidate,
          i18n.resolvedLanguage ?? i18n.language,
        ),
      );
    } catch (err) {
      console.error("Failed to open external component guidance details:", err);
      await error(t("externalComponentGuidance.errors.openDetails"));
    }
  };

  if (!candidate || !copyKey) {
    return null;
  }

  return (
    <AlertDialog open>
      <AlertDialogContent className="text-foreground">
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t("externalComponentGuidance.title")}
          </AlertDialogTitle>
          <AlertDialogDescription asChild>
            <div className="space-y-3 text-left">
              <GuidanceRow
                label={t("externalComponentGuidance.labels.missing")}
                value={t(
                  `externalComponentGuidance.candidates.${copyKey}.missing`,
                )}
              />
              <GuidanceRow
                label={t("externalComponentGuidance.labels.impact")}
                value={t(
                  `externalComponentGuidance.candidates.${copyKey}.impact`,
                )}
              />
              <GuidanceRow
                label={t("externalComponentGuidance.labels.action")}
                value={t(
                  `externalComponentGuidance.candidates.${copyKey}.action`,
                )}
              />
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter className="gap-2">
          <Button onClick={handleOpenDetails} type="button" variant="outline">
            <ExternalLinkIcon className="size-4" />
            {t("externalComponentGuidance.actions.openDetails")}
          </Button>
          <DropdownMenu modal={false}>
            <DropdownMenuTrigger asChild>
              <Button type="button" variant="secondary">
                <EyeOffIcon className="size-4" />
                {t("externalComponentGuidance.actions.hide")}
                <ChevronDownIcon className="size-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={() => void handleLater()}>
                {t("externalComponentGuidance.actions.hideThisSession")}
              </DropdownMenuItem>
              <DropdownMenuItem onSelect={() => void handleNeverShowAgain()}>
                {t("externalComponentGuidance.actions.neverShowAgain")}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
};

const GuidanceRow = ({ label, value }: { label: string; value: string }) => (
  <div className="space-y-1">
    <div className="font-medium text-foreground text-sm">{label}</div>
    <div className="text-muted-foreground text-sm leading-relaxed">{value}</div>
  </div>
);
