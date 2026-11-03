import { XIcon } from "@phosphor-icons/react";
import { platform } from "@tauri-apps/plugin-os";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { useSensorServiceSettings } from "@/hooks/navigation/useSensorServiceSettings";
import { useProcessElevated } from "@/hooks/settings/useElevationAvailability";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { startVisiblePolling } from "@/lib/visiblePolling";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import type { SelectedDisplayType } from "@/types/ui";

/** Preserve the saved startup choice while making its service replacement discoverable. */
export const SensorServiceStartupNotice = ({
  settingsLoaded,
  displayTarget,
}: {
  settingsLoaded: boolean;
  displayTarget: SelectedDisplayType | null;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const elevated = useProcessElevated();
  const openSettings = useSensorServiceSettings();
  const [connected, setConnected] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  const eligible =
    settingsLoaded &&
    platform() === "windows" &&
    settings.elevatedStartupMode &&
    elevated === true &&
    !dismissed;

  useEffect(() => {
    if (!eligible) return;

    let cancelled = false;
    const stop = startVisiblePolling(() => {
      void commands.getSensorServiceStatus().then(
        (result) => {
          if (!cancelled) {
            setConnected(
              !isError(result) &&
                result.data.connected &&
                result.data.hasReadings,
            );
          }
        },
        (error) => {
          console.error("Sensor service startup check failed:", error);
          if (!cancelled) setConnected(false);
        },
      );
    }, 60_000);

    return () => {
      cancelled = true;
      stop();
    };
  }, [eligible]);

  return (
    <div role="status" aria-live="polite">
      {eligible && connected && displayTarget !== "settings" && (
        <aside
          className="fixed right-4 bottom-4 z-40 flex max-w-md items-start gap-3 rounded-xl border border-border bg-background/95 p-4 shadow-lg backdrop-blur-sm"
          aria-label={t("sensorService.startupNotice.title")}
        >
          <div className="min-w-0 flex-1 space-y-2">
            <p className="font-semibold">
              {t("sensorService.startupNotice.title")}
            </p>
            <p className="text-sm text-muted-foreground">
              {t("sensorService.startupNotice.description")}
            </p>
            <Button type="button" variant="secondary" onClick={openSettings}>
              {t("sensorService.openSettings")}
            </Button>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            className="-mt-2 -mr-2 shrink-0"
            onClick={() => setDismissed(true)}
            aria-label={t("sensorService.startupNotice.dismiss")}
          >
            <XIcon size={18} />
          </Button>
        </aside>
      )}
    </div>
  );
};
