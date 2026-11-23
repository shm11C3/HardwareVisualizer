import { platform } from "@tauri-apps/plugin-os";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { LoadFailure } from "@/components/LoadFailure";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useSensorServiceSettingsFocus } from "@/hooks/navigation/useSensorServiceSettings";
import { sensorServiceRecoveryDocsUrl } from "@/lib/externalComponentGuidance";
import { openURL } from "@/lib/openUrl";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

type Status = "loading" | "connected" | "unavailable" | "failed";

export const SensorServiceSection = () => {
  const { t, i18n } = useTranslation();
  const windows = platform() === "windows";
  const [status, setStatus] = useState<Status>("loading");
  const [docsOpenFailed, setDocsOpenFailed] = useState(false);
  const sectionRef = useSensorServiceSettingsFocus(windows);

  const load = useCallback(async () => {
    setStatus("loading");
    setDocsOpenFailed(false);
    try {
      const result = await commands.getSensorServiceStatus();
      if (isError(result)) throw new Error(result.error);
      setStatus(result.data ? "connected" : "unavailable");
    } catch (error) {
      console.error("Sensor service status failed:", error);
      setStatus("failed");
    }
  }, []);

  useEffect(() => {
    if (windows) void load();
  }, [load, windows]);

  const openRecoveryGuide = async () => {
    setDocsOpenFailed(false);
    try {
      await openURL(
        sensorServiceRecoveryDocsUrl(i18n.resolvedLanguage ?? i18n.language),
      );
    } catch (error) {
      console.error(
        "Failed to open sensor service recovery documentation:",
        error,
      );
      setDocsOpenFailed(true);
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
      {status === "failed" ? (
        <LoadFailure
          message={t("sensorService.loadFailed")}
          onRetry={() => void load()}
        />
      ) : (
        <>
          <p role="status" className="text-sm">
            {t(`sensorService.${status}`)}
          </p>
          <div className="flex flex-wrap gap-2">
            {status === "unavailable" && (
              <Button
                type="button"
                variant="secondary"
                onClick={() => void openRecoveryGuide()}
              >
                {t("sensorService.recoveryGuide")}
              </Button>
            )}
            <Button
              type="button"
              variant="outline"
              disabled={status === "loading"}
              onClick={() => void load()}
            >
              {t("sensorService.refresh")}
            </Button>
          </div>
          {status === "unavailable" && docsOpenFailed && (
            <ActionFailure
              message={t(
                "pages.settings.advanced.openExternalComponentsDocsError",
              )}
            />
          )}
        </>
      )}
    </section>
  );
};
