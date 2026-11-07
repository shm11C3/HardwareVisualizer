import { platform } from "@tauri-apps/plugin-os";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { LoadFailure } from "@/components/LoadFailure";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useSensorServiceSettingsFocus } from "@/hooks/navigation/useSensorServiceSettings";
import { commands, type SensorServiceStatus } from "@/rspc/bindings";
import { isError } from "@/types/result";

type StatusRead =
  | { state: "loading" }
  | { state: "ready"; data: SensorServiceStatus }
  | { state: "failed" };

export const SensorServiceSection = () => {
  const { t } = useTranslation();
  const windows = platform() === "windows";
  const [read, setRead] = useState<StatusRead>({ state: "loading" });
  const [setupFailed, setSetupFailed] = useState(false);
  const [busy, setBusy] = useState(false);
  const status = read.state === "ready" ? read.data : null;
  const sectionRef = useSensorServiceSettingsFocus(windows);

  const load = useCallback(async () => {
    setRead({ state: "loading" });
    try {
      const result = await commands.getSensorServiceStatus();
      if (isError(result)) throw new Error(result.error);
      setRead({ state: "ready", data: result.data });
    } catch (error) {
      console.error("Sensor service status failed:", error);
      setRead({ state: "failed" });
    }
  }, []);

  useEffect(() => {
    if (windows) void load();
  }, [load, windows]);

  const configure = async () => {
    setBusy(true);
    setSetupFailed(false);
    try {
      const result = await commands.configureSensorService();
      if (isError(result)) throw new Error(result.error);
      await load();
    } catch (error) {
      console.error("Sensor service setup failed:", error);
      setSetupFailed(true);
    } finally {
      setBusy(false);
    }
  };

  if (!windows) return null;

  return (
    <section
      ref={sectionRef}
      tabIndex={-1}
      className="w-full scroll-mt-6 space-y-3 py-6 xl:w-1/2"
      aria-labelledby="sensor-service-title"
    >
      <Label id="sensor-service-title" className="text-lg">
        {t("sensorService.title")}
      </Label>
      <p className="text-sm text-muted-foreground">
        {t("sensorService.description")}
      </p>
      <p className="text-sm text-muted-foreground">
        {t("sensorService.setupSteps", {
          section: t("pages.settings.advanced.externalComponentSetup.title"),
        })}
      </p>
      {read.state === "failed" ? (
        <LoadFailure
          message={t("sensorService.loadFailed")}
          onRetry={() => void load()}
        />
      ) : (
        <p role="status" className="text-sm">
          {t(
            status
              ? status.connected
                ? "sensorService.connected"
                : "sensorService.unavailable"
              : "sensorService.loading",
          )}
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
          disabled={busy || read.state === "loading"}
          onClick={() => void load()}
        >
          {t("sensorService.refresh")}
        </Button>
      </div>
      {setupFailed && <ActionFailure message={t("sensorService.failed")} />}
    </section>
  );
};
