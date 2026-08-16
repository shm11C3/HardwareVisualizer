import { useAtomValue } from "jotai";
import { useTranslation } from "react-i18next";
import {
  motherboardFanSpeedsAtom,
  motherboardTempsAtom,
} from "@/features/hardware/store/chart";
import type { FanSpeedStatus } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

/**
 * Live Super I/O readings on the Performance Tab. The static motherboard
 * facts stay on the System Specifications sheet; this panel only carries the
 * values that change while you watch.
 */
export const MotherboardSensorsPanel = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const motherboardTemps = useAtomValue(motherboardTempsAtom);
  const motherboardFanSpeeds = useAtomValue(motherboardFanSpeedsAtom);
  const temperatureUnit = settings.temperatureUnit === "C" ? "°C" : "°F";

  if (motherboardTemps.length === 0 && motherboardFanSpeeds.length === 0) {
    return (
      <p className="px-4 pb-4 text-muted-foreground text-sm">
        {t("pages.performance.motherboardSensorsUnavailable")}
      </p>
    );
  }

  const sensorSource =
    motherboardTemps[0]?.source ?? motherboardFanSpeeds[0]?.source;
  const fanStatusLabel = (status: FanSpeedStatus) => {
    switch (status) {
      case "active":
        return t("pages.dashboard.motherboardSensors.status.active");
      case "inactive":
        return t("pages.dashboard.motherboardSensors.status.inactive");
      case "invalid":
        return t("pages.dashboard.motherboardSensors.status.invalid");
    }
  };

  return (
    <div className="space-y-3 p-4 pt-2">
      {sensorSource != null && (
        <span className="inline-block rounded-sm bg-muted/80 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
          {sensorSource}
        </span>
      )}
      {/* Same width-driven rule as the per-core panel: the sensor list must
          reflow on the panel's width, not the window's. */}
      <div className="grid grid-cols-[repeat(auto-fill,minmax(13rem,1fr))] gap-x-8 gap-y-1">
        {motherboardTemps.map((sensor) => (
          <div
            key={sensor.name}
            className="flex items-baseline justify-between gap-4 border-border/60 border-b py-1.5 text-sm"
          >
            <span className="text-muted-foreground">{sensor.name}</span>
            <span className="font-mono tabular-nums">
              {sensor.value} {temperatureUnit}
            </span>
          </div>
        ))}
        {motherboardFanSpeeds.map((fan) => (
          <div
            key={fan.name}
            className="flex items-baseline justify-between gap-4 border-border/60 border-b py-1.5 text-sm"
          >
            <span className="text-muted-foreground">{fan.name}</span>
            <span className="font-mono tabular-nums">
              {fan.rpm != null
                ? `${fan.rpm} RPM (${fanStatusLabel(fan.status)})`
                : `${t("shared.notAvailable")} (${fanStatusLabel(fan.status)})`}
            </span>
          </div>
        ))}
      </div>
    </div>
  );
};
