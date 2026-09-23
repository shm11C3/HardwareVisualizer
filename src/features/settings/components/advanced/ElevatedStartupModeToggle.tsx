import { platform } from "@tauri-apps/plugin-os";
import { ShieldIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { useElevationAvailability } from "@/hooks/useElevationAvailability";

export const ElevatedStartupModeToggle = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const availability = useElevationAvailability();

  if (platform() !== "windows") {
    return null;
  }

  // Outside Program Files the app refuses to elevate (#2216). A saved "on" is
  // kept and can still be turned off; it just cannot be turned on here.
  const unprotected = availability === "unprotectedLocation";

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
          {unprotected && (
            <p className="text-amber-600 text-sm dark:text-amber-400">
              {settings.elevatedStartupMode
                ? t("elevationUnavailable.elevatedStartupModeNotApplied")
                : t("elevationUnavailable.reason")}
            </p>
          )}
        </div>
      </div>

      <Switch
        id="elevatedStartupMode"
        checked={settings.elevatedStartupMode}
        disabled={unprotected && !settings.elevatedStartupMode}
        onCheckedChange={(value) =>
          updateSettingAtom("elevatedStartupMode", value)
        }
      />
    </div>
  );
};
