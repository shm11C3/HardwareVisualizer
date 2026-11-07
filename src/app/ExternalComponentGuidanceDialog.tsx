import { platform } from "@tauri-apps/plugin-os";
import {
  ChevronDownIcon,
  ExternalLinkIcon,
  EyeOffIcon,
  ShieldIcon,
} from "lucide-react";
import { useEffect, useLayoutEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { HardwareCompatibilityGuideLink } from "@/components/HardwareCompatibilityGuideLink";
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
import {
  elevationUnavailableReasonKey,
  useElevationAvailability,
  useProcessElevated,
} from "@/hooks/settings/useElevationAvailability";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { useTauriDialog } from "@/hooks/tauri/useTauriDialog";
import {
  externalComponentGuidanceActionKey,
  externalComponentGuidanceCopyKey,
  externalComponentGuidanceDocsUrl,
} from "@/lib/externalComponentGuidance";
import { openURL } from "@/lib/openUrl";
import { startVisiblePolling } from "@/lib/visiblePolling";
import type { ExternalComponentGuidanceCandidate } from "@/rspc/bindings";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import type { SelectedDisplayType } from "@/types/ui";

type ExternalComponentGuidanceDialogProps = {
  displayTarget: SelectedDisplayType | null;
  settingsLoaded?: boolean;
  onRequestChange?: (requested: boolean) => void;
  /** Another startup AlertDialog is open; keep the candidate, wait to show. */
  deferred?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** Whether it is still unknown if guidance will open for this screen. */
  onPendingChange?: (pending: boolean) => void;
};

export const ExternalComponentGuidanceDialog = ({
  displayTarget,
  settingsLoaded = true,
  onRequestChange,
  deferred = false,
  onOpenChange,
  onPendingChange,
}: ExternalComponentGuidanceDialogProps) => {
  const { t, i18n } = useTranslation();
  const { error } = useTauriDialog();
  const { updateSettingAtom } = useSettingsAtom();
  const [candidates, setCandidates] = useState<
    ExternalComponentGuidanceCandidate[]
  >([]);
  const [isEnablingElevatedStartupMode, setIsEnablingElevatedStartupMode] =
    useState(false);
  // The view whose first candidate lookup has completed. Derived per render
  // rather than a boolean flipped in the effect, so a screen change counts
  // as pending in the very render it happens, before any effect runs.
  const [loadedView, setLoadedView] = useState<"dashboard" | null>(null);

  const view = displayTarget === "dashboard" ? "dashboard" : null;
  const screenKnown = displayTarget !== null;
  const lookupPending =
    !settingsLoaded || !screenKnown || (view !== null && loadedView !== view);

  useEffect(() => {
    if (!settingsLoaded || !screenKnown || !view) {
      setCandidates([]);
      return;
    }
    let cancelled = false;
    const load = async () => {
      try {
        const result = await commands.getExternalComponentGuidanceCandidates();
        if (cancelled) return;
        if (isError(result)) throw new Error(result.error);
        setCandidates(
          result.data.filter((item) => item.component === "smartctl"),
        );
      } catch (error) {
        if (!cancelled)
          console.error("External component guidance failed:", error);
      } finally {
        if (!cancelled) setLoadedView(view);
      }
    };
    const stop = startVisiblePolling(() => void load(), 60_000);
    return () => {
      cancelled = true;
      stop();
    };
  }, [settingsLoaded, screenKnown, view]);

  const processElevated = useProcessElevated();
  const candidate = loadedView === view ? (candidates[0] ?? null) : null;
  const elevationAvailability = useElevationAvailability();
  // Outside Program Files the app refuses to elevate (#2216), so offer the
  // details page instead of a restart that would fail.
  const isWindowsPermissionGuidance =
    candidate?.reasonKind === "permission" && platform() === "windows";
  const shouldShowElevatedStartupAction =
    isWindowsPermissionGuidance &&
    processElevated === false &&
    elevationAvailability === "available";
  // The permission copy suggests running as administrator, which is refused
  // here, so say why instead.
  const elevationReasonKey = isWindowsPermissionGuidance
    ? elevationUnavailableReasonKey(elevationAvailability)
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

  const elevationPending =
    isWindowsPermissionGuidance && elevationAvailability === null;
  const wouldOpen = Boolean(candidate && !elevationPending);
  const isOpen = wouldOpen && !deferred;
  // A candidate that is ready but held back still counts as pending: it will
  // open the moment the blocker closes, so dialogs that yield to this one
  // must not take that same render.
  const pending = lookupPending || elevationPending || (wouldOpen && deferred);

  useLayoutEffect(() => {
    onRequestChange?.(wouldOpen);
  }, [wouldOpen, onRequestChange]);

  useLayoutEffect(() => {
    onOpenChange?.(isOpen);
  }, [isOpen, onOpenChange]);
  useLayoutEffect(() => {
    onPendingChange?.(pending);
  }, [pending, onPendingChange]);

  const handleEnableElevatedStartupMode = async () => {
    if (!candidate || !shouldShowElevatedStartupAction) return;

    setIsEnablingElevatedStartupMode(true);
    try {
      // Share the Settings toggle's persistence and rollback behavior.
      // The existing command owns the elevated relaunch and UAC cancellation;
      // this modal reports a failed update once, in translated copy.
      if (!(await updateSettingAtom("elevatedStartupMode", true))) {
        await error(t("externalComponentGuidance.errors.elevatedStartupMode"));
      }
    } catch (err) {
      console.error(
        "Failed to enable elevated startup mode from external component guidance:",
        err,
      );
      await error(t("externalComponentGuidance.errors.elevatedStartupMode"));
    } finally {
      setIsEnablingElevatedStartupMode(false);
    }
  };

  if (!isOpen || !candidate) {
    return null;
  }

  const copyKey = externalComponentGuidanceCopyKey(candidate);
  const actionKey = externalComponentGuidanceActionKey(candidate);

  return (
    <AlertDialog open>
      <AlertDialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto text-foreground">
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
                value={t(elevationReasonKey ?? actionKey)}
              />
              {shouldShowElevatedStartupAction && (
                <p>{t("externalComponentGuidance.elevatedStartupConsent")}</p>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <HardwareCompatibilityGuideLink />
        <AlertDialogFooter className="gap-2">
          {shouldShowElevatedStartupAction ? (
            <Button
              disabled={isEnablingElevatedStartupMode}
              onClick={() => void handleEnableElevatedStartupMode()}
              type="button"
              variant="outline"
            >
              <ShieldIcon className="size-4" />
              {t("externalComponentGuidance.actions.enableElevatedStartupMode")}
            </Button>
          ) : (
            <Button onClick={handleOpenDetails} type="button" variant="outline">
              <ExternalLinkIcon className="size-4" />
              {t("externalComponentGuidance.actions.openDetails")}
            </Button>
          )}
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
    <div className="text-sm font-medium text-foreground">{label}</div>
    <div className="text-sm leading-relaxed text-muted-foreground">{value}</div>
  </div>
);
