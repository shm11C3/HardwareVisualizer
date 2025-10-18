import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { ClientSettings } from "@/rspc/bindings";

export const GraphStyleToggle = ({
  type,
}: {
  type: Extract<
    keyof ClientSettings,
    | "lineGraphBorder"
    | "lineGraphFill"
    | "lineGraphMix"
    | "lineGraphShowLegend"
    | "lineGraphShowScale"
    | "lineGraphShowTooltip"
  >;
}) => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const settingGraphSwitch = async (value: boolean) => {
    await updateSettingAtom(type, value);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center space-x-4 py-3">
        <div className="flex w-full flex-row items-center justify-between rounded-lg border border-zinc-800 p-4 dark:border-gray-100">
          <div className="space-y-0.5">
            <Label htmlFor={type} className="text-lg">
              {t(`pages.settings.customTheme.graphStyle.${type}`)}
            </Label>
          </div>

          <Switch
            checked={settings[type]}
            onCheckedChange={settingGraphSwitch}
          />
        </div>
      </div>
    </div>
  );
};
