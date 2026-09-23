import { platform } from "@tauri-apps/plugin-os";
import { ShieldIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import {
  elevationUnavailableReasonKey,
  useElevationAvailability,
  useProcessElevated,
} from "@/hooks/useElevationAvailability";

export const ElevatedStartupModeToggle = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const availability = useElevationAvailability();
  const processElevated = useProcessElevated();

  if (platform() !== "windows") {
    return null;
  }

  // Elevation is offered only when the backend positively reports it (#2216).
  // A saved "on" is kept and can always be turned off.
  const canEnable = availability === "available";
  const reasonKey = elevationUnavailableReasonKey(availability);
  // "Not applied" only when this launch really ran unelevated because of the
  // install folder; a process the user started as administrator did apply it.
  const notApplied =
    settings.elevatedStartupMode &&
    availability === "unprotectedLocation" &&
    processElevated === false;
  const note = notApplied
    ? t("elevationUnavailable.elevatedStartupModeNotApplied")
    : reasonKey
      ? t(reasonKey)
      : null;

  return (
    <div className="flex w-full items-center justify-between gap-4 py-6 xl:w-1/2">
      <div className="flex items-start gap-3">
        <ShieldIcon className="mt-1 size-5 shrink-0 text-muted-foreground" />
        <div className="space-y-1">
          <Label htmlFor="elevatedStartupMode" className="text-lg">
            {t("pages.settings.advanced.elevatedStartupMode.name")}
          </Label>
          <p className="text-muted-foreground text-sm">
            {t("pages.settings.advanced.elevatedStartupMode.description")}
          </p>
          {note && (
            <p className="text-amber-600 text-sm dark:text-amber-400">{note}</p>
          )}
        </div>
      </div>

      <Switch
        id="elevatedStartupMode"
        checked={settings.elevatedStartupMode}
        disabled={!canEnable && !settings.elevatedStartupMode}
        onCheckedChange={(value) =>
          updateSettingAtom("elevatedStartupMode", value)
        }
      />
    </div>
  );
};
