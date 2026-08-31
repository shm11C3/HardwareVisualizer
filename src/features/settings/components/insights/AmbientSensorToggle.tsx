import { platform } from "@tauri-apps/plugin-os";
import { ThermometerIcon } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { NeedRestart } from "@/components/shared/System";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

/**
 * Opt-in for the SwitchBot Meter ambient source (#2044).
 *
 * Hidden outside Windows because no other platform has a BLE transport
 * yet — showing a switch that cannot start a scan would promise a
 * reading the app has no way to take.
 *
 * The description carries the placement guidance rather than hiding it
 * behind a tooltip or a docs link. Where the sensor sits decides whether
 * its readings mean anything: a meter in the exhaust stream measures the
 * machine's own waste heat and reports a Thermal Delta that mostly
 * tracks the fan curve, and the person about to place one is reading
 * this line at exactly the moment the advice is actionable.
 */
export const AmbientSensorToggle = () => {
  const [alertOpen, setAlertOpen] = useState(false);
  const { t } = useTranslation();
  const { settings, toggleSwitchbotMeterAtom } = useSettingsAtom();

  if (platform() !== "windows") {
    return null;
  }

  const handleCheckedChange = async (value: boolean) => {
    // Only prompt for a restart once the preference is actually on disk.
    // A refused write leaves the scan as it was, so the restart notice
    // would be telling the user to apply a change that never happened —
    // on top of the error dialog the failed write already raised.
    if (await toggleSwitchbotMeterAtom(value)) {
      setAlertOpen(true);
    }
  };

  return (
    <>
      <div className="flex w-full items-start justify-between gap-4 py-6">
        <div className="flex items-start gap-3">
          <ThermometerIcon className="mt-1 size-5 shrink-0 text-muted-foreground" />
          <div className="space-y-1">
            <Label htmlFor="switchbotMeter" className="text-lg">
              {t("pages.settings.insights.ambientSensor.name")}
            </Label>
            <p className="text-muted-foreground text-sm">
              {t("pages.settings.insights.ambientSensor.description")}
            </p>
            <p className="text-muted-foreground text-sm">
              {t("pages.settings.insights.ambientSensor.placement")}
            </p>
          </div>
        </div>

        <Switch
          id="switchbotMeter"
          checked={settings.environmentalSensors.switchbotMeterEnabled ?? false}
          onCheckedChange={handleCheckedChange}
        />
      </div>
      <NeedRestart alertOpen={alertOpen} setAlertOpen={setAlertOpen} />
    </>
  );
};
