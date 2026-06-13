import { platform } from "@tauri-apps/plugin-os";
import { ShieldIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

export const ElevatedStartupModeToggle = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();

  if (platform() !== "windows") {
    return null;
  }

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
        </div>
      </div>

      <Switch
        id="elevatedStartupMode"
        checked={settings.elevatedStartupMode}
        onCheckedChange={(value) =>
          updateSettingAtom("elevatedStartupMode", value)
        }
      />
    </div>
  );
};
