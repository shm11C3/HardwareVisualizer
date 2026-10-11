import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import type { TemperatureUnit } from "@/rspc/bindings";

export const TemperatureUnitSelect = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const changeTemperatureUnit = async (value: TemperatureUnit) => {
    await updateSettingAtom("temperatureUnit", value);
  };

  return (
    <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
      <Label htmlFor="temperatureUnit" className="text-lg">
        {t("pages.settings.general.temperatureUnit.name")}
      </Label>
      <Select
        value={settings.temperatureUnit}
        onValueChange={changeTemperatureUnit}
      >
        <SelectTrigger className="w-[180px]">
          <SelectValue placeholder="Temperature Unit" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="C">
            {t("pages.settings.general.temperatureUnit.celsius")}
          </SelectItem>
          <SelectItem value="F">
            {t("pages.settings.general.temperatureUnit.fahrenheit")}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
};
