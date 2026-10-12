import { AlertTriangleIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { SettingWriteFailure } from "@/components/SettingWriteFailure";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const CloseToTrayToggle = () => {
  const { t } = useTranslation();
  const { settings, setCloseToTrayPreferenceAtom } = useSettingsAtom();
  const [isAvailable, setIsAvailable] = useState(true);

  useEffect(() => {
    const checkAvailability = async () => {
      try {
        const result = await commands.isCloseToTrayAvailable();

        if (isError(result)) {
          setIsAvailable(false);
          console.error(
            "Failed to check close-to-tray availability:",
            result.error,
          );
          return;
        }

        setIsAvailable(result.data);
      } catch (err) {
        setIsAvailable(false);
        console.error("Failed to check close-to-tray availability:", err);
      }
    };

    checkAvailability();
  }, []);

  const updateCloseToTray = async (value: boolean) => {
    // A failed save is recorded for this row (`SettingWriteFailure`).
    await setCloseToTrayPreferenceAtom(value);
  };

  return (
    <div className="space-y-3 py-6 xl:w-1/2">
      <div className="flex w-full items-center justify-between gap-4">
        <div className="space-y-1">
          <Label htmlFor="closeToTray" className="text-lg">
            {t("pages.settings.general.closeToTray.name")}
          </Label>
          <p className="text-muted-foreground text-sm">
            {t("pages.settings.general.closeToTray.description")}
          </p>
        </div>

        <Switch
          id="closeToTray"
          checked={isAvailable && settings.closeToTray}
          disabled={!isAvailable}
          onCheckedChange={updateCloseToTray}
        />
      </div>
      <SettingWriteFailure keys="closeToTray" />

      {!isAvailable && (
        <div className="flex items-start gap-2 rounded-md border border-yellow-500/40 bg-yellow-500/10 p-3 text-sm">
          <AlertTriangleIcon className="mt-0.5 size-4 shrink-0 text-yellow-600" />
          <p>{t("pages.settings.general.closeToTray.unavailable")}</p>
        </div>
      )}
    </div>
  );
};
