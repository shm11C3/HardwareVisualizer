import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "../../hooks/useSettingsAtom";

export const TextSelectionToggle = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();

  return (
    <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
      <div className="space-y-0.5">
        <Label htmlFor="textSelection" className="text-lg">
          {t("pages.settings.general.textSelection.name")}
        </Label>
      </div>

      <Switch
        id="textSelection"
        checked={settings.textSelectable}
        onCheckedChange={(value) => updateSettingAtom("textSelectable", value)}
      />
    </div>
  );
};
