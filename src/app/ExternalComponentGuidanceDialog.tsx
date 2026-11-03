import { platform } from "@tauri-apps/plugin-os";
import {
  ChevronDownIcon,
  ExternalLinkIcon,
  EyeOffIcon,
  ShieldIcon,
} from "lucide-react";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
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
import { useSensorServiceSettings } from "@/hooks/navigation/useSensorServiceSettings";
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
  externalComponentGuidanceViewForDisplayTarget,
  type PerformanceGuidanceContext,
} from "@/lib/externalComponentGuidance";
import { openURL } from "@/lib/openUrl";
import { startVisiblePolling } from "@/lib/visiblePolling";
import type {
  ExternalComponentGuidanceCandidate,
  ExternalComponentGuidanceView,
} from "@/rspc/bindings";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import type { SelectedDisplayType } from "@/types/ui";

type ExternalComponentGuidanceDialogProps = {
  displayTarget: SelectedDisplayType | null;
  settingsLoaded?: boolean;
  performanceContext?: PerformanceGuidanceContext;
  onRequestChange?: (requested: boolean) => void;
  /** Another startup AlertDialog is open; keep the candidate, wait to show. */
  deferred?: boolean;
  onOpenChange?: (open: boolean) => void;
  /** Whether it is still unknown if guidance will open for this screen. */
  onPendingChange?: (pending: boolean) => void;
};

// Allow the initial collector/service connection to settle, without opening
// installation dialogs later during normal use or after restoring the window.
const SENSOR_STARTUP_WINDOW_MS = 10_000;

const sensorCandidateIsVisible = (
  item: ExternalComponentGuidanceCandidate,
  view: ExternalComponentGuidanceView | null,
  context?: PerformanceGuidanceContext,
) => {
  if (item.component !== "pawnio") return false;
  if (view === "dashboard") return true;
  if (view === "cpuDetail") return item.usage === "cpuPackageTemperature";
  if (view !== "performance" || !context?.settled || context.expanded)
    return false;

  return (
    (item.usage === "cpuPackageTemperature" && context.cpuTemperatureVisible) ||
    (item.usage === "motherboardSensors" && context.motherboardSensorsVisible)
  );
};

export const ExternalComponentGuidanceDialog = ({
  displayTarget,
  settingsLoaded = true,
  performanceContext,
  onRequestChange,
  deferred = false,
  onOpenChange,
  onPendingChange,
}: ExternalComponentGuidanceDialogProps) => {
  const { t, i18n } = useTranslation();
  const { error } = useTauriDialog();
  const { updateSettingAtom } = useSettingsAtom();
  const openSensorServiceSettings = useSensorServiceSettings();
  const [candidates, setCandidates] = useState<
    ExternalComponentGuidanceCandidate[]
  >([]);
  const [startupSensor, setStartupSensor] = useState<{
    view: ExternalComponentGuidanceView;
    candidate: ExternalComponentGuidanceCandidate;
  } | null>(null);
  const sensorStartup = useRef<{
    view: ExternalComponentGuidanceView | null;
    deadline: number;
  } | null>(null);
  const sensorOffered = useRef(false);
  const sensorPresented = useRef(false);
  const [isEnablingElevatedStartupMode, setIsEnablingElevatedStartupMode] =
    useState(false);
  // The view whose first candidate lookup has completed. Derived per render
  // rather than a boolean flipped in the effect, so a screen change counts
  // as pending in the very render it happens, before any effect runs.
  const [loadedView, setLoadedView] =
    useState<ExternalComponentGuidanceView | null>(null);

  const view = useMemo(
    () => externalComponentGuidanceViewForDisplayTarget(displayTarget),
    [displayTarget],
  );
  const screenKnown = displayTarget !== null;
  const lookupPending =
    !settingsLoaded || !screenKnown || (view !== null && loadedView !== view);

  useEffect(() => {
    if (!settingsLoaded || !screenKnown || !view) {
      setCandidates([]);
      setStartupSensor(null);
      if (settingsLoaded && screenKnown) {
        sensorStartup.current = { view, deadline: 0 };
      }
      return;
    }

    sensorStartup.current ??= {
      view,
      deadline: Date.now() + SENSOR_STARTUP_WINDOW_MS,
    };
    if (sensorStartup.current.view !== view) sensorStartup.current.deadline = 0;
    setStartupSensor((current) =>
      current &&
      (current.view !== view ||
        !sensorCandidateIsVisible(current.candidate, view, performanceContext))
        ? null
        : current,
    );

    let isCancelled = false;
    let startupRetry: ReturnType<typeof setTimeout> | undefined;
    const startupExpiry = setTimeout(
      () => {
        // A prompt already being read remains under the user's control. A
        // queued prompt must not interrupt normal use after another flow ends.
        if (!sensorPresented.current) setStartupSensor(null);
      },
      Math.max(0, sensorStartup.current.deadline - Date.now()),
    );
    const startupCheckOpen = () =>
      platform() === "windows" &&
      !document.hidden &&
      !sensorOffered.current &&
      sensorStartup.current?.view === view &&
      Date.now() < sensorStartup.current.deadline;

    const loadCandidates = async () => {
      if (isCancelled) return;

      // Only Dashboard has unrelated storage guidance that still needs live
      // polling. CPU/Performance sensor guidance ends with the startup window.
      if (view !== "dashboard" && !startupCheckOpen()) {
        setLoadedView(view);
        return;
      }

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

        setCandidates(
          view === "dashboard"
            ? result.data.filter((item) => item.component !== "pawnio")
            : [],
        );

        if (startupCheckOpen()) {
          const sensor = result.data.find((item) =>
            sensorCandidateIsVisible(item, view, performanceContext),
          );

          if (sensor) {
            sensorOffered.current = true;
            setStartupSensor({ view, candidate: sensor });
          }
        }
      } catch (err) {
        if (isCancelled) return;
        console.error("Failed to fetch external component guidance:", err);
      } finally {
        if (!isCancelled) {
          setLoadedView(view);
          if (startupCheckOpen()) {
            clearTimeout(startupRetry);
            startupRetry = setTimeout(() => void loadCandidates(), 1_000);
          }
        }
      }
    };

    let stopPolling = () => {};
    if (view === "dashboard") {
      stopPolling = startVisiblePolling(() => void loadCandidates(), 60_000);
    } else {
      void loadCandidates();
    }

    return () => {
      isCancelled = true;
      clearTimeout(startupRetry);
      clearTimeout(startupExpiry);
      stopPolling();
    };
  }, [settingsLoaded, screenKnown, view, performanceContext]);

  const processElevated = useProcessElevated();
  const isPerformance = view === "performance";
  const performanceReady =
    !isPerformance || Boolean(performanceContext?.settled);
  const candidate =
    (startupSensor?.view === view &&
    sensorCandidateIsVisible(startupSensor.candidate, view, performanceContext)
      ? startupSensor.candidate
      : null) ??
    (loadedView === view ? candidates[0] : null) ??
    null;
  const copyKey = candidate
    ? externalComponentGuidanceCopyKey(candidate)
    : null;
  const actionKey = candidate
    ? externalComponentGuidanceActionKey(candidate)
    : null;
  const elevationAvailability = useElevationAvailability();
  // Outside Program Files the app refuses to elevate (#2216), so offer the
  // details page instead of a restart that would fail.
  const isWindowsPermissionGuidance =
    candidate?.reasonKind === "permission" &&
    candidate.component !== "pawnio" &&
    platform() === "windows";
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
    setStartupSensor((current) =>
      current?.candidate.key === key ? null : current,
    );
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
  const wouldOpen = Boolean(
    candidate && copyKey && actionKey && !elevationPending,
  );
  const isOpen = wouldOpen && !deferred;
  // A candidate that is ready but held back still counts as pending: it will
  // open the moment the blocker closes, so dialogs that yield to this one
  // must not take that same render.
  const pending =
    lookupPending ||
    !performanceReady ||
    elevationPending ||
    (wouldOpen && deferred);

  useLayoutEffect(() => {
    onRequestChange?.(wouldOpen);
  }, [wouldOpen, onRequestChange]);

  // Before paint, so App defers other dialogs in the same frame.
  useLayoutEffect(() => {
    if (isOpen && candidate?.component === "pawnio")
      sensorPresented.current = true;
    onOpenChange?.(isOpen);
  }, [isOpen, candidate?.component, onOpenChange]);
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

  if (!isOpen || !candidate || !copyKey || !actionKey) {
    return null;
  }

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
                value={t(
                  elevationReasonKey ??
                    `externalComponentGuidance.candidates.${copyKey}.${actionKey}`,
                )}
              />
              {shouldShowElevatedStartupAction && (
                <p>{t("externalComponentGuidance.elevatedStartupConsent")}</p>
              )}
            </div>
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter className="gap-2">
          {candidate.component === "pawnio" && platform() === "windows" ? (
            <Button onClick={openSensorServiceSettings} type="button">
              {t("sensorService.openSettings")}
            </Button>
          ) : shouldShowElevatedStartupAction ? (
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
