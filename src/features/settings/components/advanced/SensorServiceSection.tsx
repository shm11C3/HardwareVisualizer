import { platform } from "@tauri-apps/plugin-os";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { SettingWriteFailure } from "@/components/SettingWriteFailure";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useProcessElevated } from "@/hooks/settings/useElevationAvailability";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { commands, type SensorServiceStatus } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const SensorServiceSection = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const elevated = useProcessElevated();
  const windows = platform() === "windows";
  const [status, setStatus] = useState<SensorServiceStatus | null>(null);
  const [failed, setFailed] = useState(false);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    setFailed(false);
    try {
      const result = await commands.getSensorServiceStatus();
      if (isError(result)) throw new Error(result.error);
      setStatus(result.data);
    } catch (error) {
      console.error("Sensor service status failed:", error);
      setFailed(true);
    }
  }, []);

  useEffect(() => {
    if (windows) void load();
  }, [load, windows]);

  const configure = async () => {
    setBusy(true);
    setFailed(false);
    try {
      const result = await commands.configureSensorService();
      if (isError(result)) throw new Error(result.error);
      await load();
    } catch (error) {
      console.error("Sensor service setup failed:", error);
      setFailed(true);
    } finally {
      setBusy(false);
    }
  };

  if (!windows) return null;

  return (
    <section
      className="w-full space-y-3 py-6 xl:w-1/2"
      aria-labelledby="sensor-service-title"
    >
      <Label id="sensor-service-title" className="text-lg">
        {t("sensorService.title")}
      </Label>
      <p className="text-sm text-muted-foreground">
        {t("sensorService.description")}
      </p>
      <p role="status" className="text-sm">
        {t(
          status
            ? status.connected
              ? "sensorService.connected"
              : "sensorService.unavailable"
            : "sensorService.loading",
        )}
      </p>
      {status?.connected && !status.hasReadings && (
        <p className="text-sm text-muted-foreground">
          {t("sensorService.noReadings")}
        </p>
      )}
      {status && !status.canConfigure && (
        <p className="text-sm text-muted-foreground">
          {t("sensorService.requiresMsi")}
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          variant="secondary"
          disabled={busy || !status?.canConfigure}
          onClick={() => void configure()}
        >
          {t(busy ? "sensorService.installing" : "sensorService.configure")}
        </Button>
        <Button
          type="button"
          variant="outline"
          disabled={busy}
          onClick={() => void load()}
        >
          {t("sensorService.refresh")}
        </Button>
      </div>
      {failed && <ActionFailure message={t("sensorService.failed")} />}
      {settings.elevatedStartupMode && (
        <div className="space-y-2">
          <p className="text-sm text-muted-foreground">
            {t("sensorService.switchDescription")}
          </p>
          <Button
            type="button"
            variant="secondary"
            disabled={!status?.connected || busy}
            onClick={() => void updateSettingAtom("elevatedStartupMode", false)}
          >
            {t("sensorService.switchToNormal")}
          </Button>
          <SettingWriteFailure actions="elevatedStartupMode" />
        </div>
      )}
      {elevated === true && (
        <p className="text-sm text-muted-foreground">
          {t("sensorService.currentProcessElevated")}
        </p>
      )}
    </section>
  );
};
