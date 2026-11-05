import { XIcon } from "@phosphor-icons/react";
import { platform } from "@tauri-apps/plugin-os";
import { useAtomValue } from "jotai";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { Button } from "@/components/ui/button";
import { sensorGuidanceAtom } from "@/features/hardware/store/sensors";
import { useSensorServiceSettings } from "@/hooks/navigation/useSensorServiceSettings";
import { useProcessElevated } from "@/hooks/settings/useElevationAvailability";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import {
  externalComponentGuidanceCopyKey,
  type PerformanceGuidanceContext,
} from "@/lib/externalComponentGuidance";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import type { SelectedDisplayType } from "@/types/ui";

export const SensorServiceNotice = ({
  settingsLoaded,
  displayTarget,
  performanceContext,
}: {
  settingsLoaded: boolean;
  displayTarget: SelectedDisplayType | null;
  performanceContext: PerformanceGuidanceContext;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const elevated = useProcessElevated();
  const diagnostics = useAtomValue(sensorGuidanceAtom);
  const openSettings = useSensorServiceSettings();
  const [dismissed, setDismissed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<"acknowledge" | "defer" | null>(null);
  const candidates = diagnostics.filter((item) => {
    if (item.component !== "pawnio") return false;
    if (displayTarget === "dashboard") return true;
    if (displayTarget === "cpuDetail")
      return item.usage === "cpuPackageTemperature";
    if (
      displayTarget !== "performance" ||
      !performanceContext.settled ||
      performanceContext.expanded
    )
      return false;
    return (
      (item.usage === "cpuPackageTemperature" &&
        performanceContext.cpuTemperatureVisible) ||
      (item.usage === "motherboardSensors" &&
        performanceContext.motherboardSensorsVisible)
    );
  });
  const needsSetup = candidates.length > 0;
  const visible =
    settingsLoaded &&
    platform() === "windows" &&
    displayTarget !== null &&
    displayTarget !== "settings" &&
    !dismissed &&
    (needsSetup || (settings.elevatedStartupMode && elevated === true));

  const dismiss = async (permanently: boolean) => {
    setBusy(true);
    setFailure(null);
    try {
      const save = permanently
        ? commands.acknowledgeExternalComponentGuidanceKey
        : commands.deferExternalComponentGuidanceForSession;
      for (const candidate of candidates) {
        const result = await save(candidate.key);
        if (isError(result)) throw new Error(result.error);
      }
      setDismissed(true);
    } catch (error) {
      console.error("Sensor guidance dismissal failed:", error);
      setFailure(permanently ? "acknowledge" : "defer");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div role="status" aria-live="polite">
      {visible && (
        <aside
          className="flex items-start gap-3 rounded-xl border border-border bg-background/95 p-4 shadow-lg backdrop-blur-sm"
          aria-label={t(
            needsSetup
              ? "sensorService.notice.setupTitle"
              : "sensorService.notice.permissionsTitle",
          )}
        >
          <div className="min-w-0 flex-1 space-y-2">
            <p className="font-semibold">
              {t(
                needsSetup
                  ? "sensorService.notice.setupTitle"
                  : "sensorService.notice.permissionsTitle",
              )}
            </p>
            {needsSetup && (
              <p className="text-sm">
                {candidates
                  .map((item) =>
                    t(
                      `externalComponentGuidance.candidates.${externalComponentGuidanceCopyKey(item)}.impact`,
                    ),
                  )
                  .join(" / ")}
              </p>
            )}
            <p className="text-sm text-muted-foreground">
              {t(
                needsSetup
                  ? "sensorService.setupGuidance"
                  : "sensorService.notice.permissionsDescription",
              )}
            </p>
            <div className="flex flex-wrap gap-2">
              <Button type="button" variant="secondary" onClick={openSettings}>
                {t("sensorService.openSettings")}
              </Button>
              {needsSetup && (
                <Button
                  type="button"
                  variant="ghost"
                  disabled={busy}
                  onClick={() => void dismiss(true)}
                >
                  {t("externalComponentGuidance.actions.neverShowAgain")}
                </Button>
              )}
            </div>
            {failure && (
              <ActionFailure
                message={t(`externalComponentGuidance.errors.${failure}`)}
              />
            )}
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            disabled={busy}
            className="-mt-2 -mr-2 shrink-0"
            onClick={() => void dismiss(false)}
            aria-label={t("sensorService.notice.dismiss")}
          >
            <XIcon size={18} />
          </Button>
        </aside>
      )}
    </div>
  );
};
