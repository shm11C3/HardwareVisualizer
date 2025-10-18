import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { Theme } from "@/rspc/bindings";

export const ThemeSelect = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const toggleDarkMode = async (mode: Theme) => {
    await updateSettingAtom("theme", mode);
  };

  return (
    <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
      <Label htmlFor="darkMode" className="text-lg">
        {t("pages.settings.general.colorMode.name")}
      </Label>
      <Select value={settings.theme} onValueChange={toggleDarkMode}>
        <SelectTrigger className="w-[180px]">
          <SelectValue placeholder="Theme" />
        </SelectTrigger>
        <SelectContent>
          <SelectItem
            value="system"
            className="focus:bg-gray-200 dark:focus:bg-gray-400"
          >
            {t("pages.settings.general.colorMode.system")}
          </SelectItem>
          <SelectItem
            value="light"
            className="focus:bg-gray-200 dark:focus:bg-gray-400"
          >
            {t("pages.settings.general.colorMode.light")}
          </SelectItem>
          <SelectItem
            value="dark"
            className="focus:bg-gray-400 dark:focus:bg-gray-700"
          >
            {t("pages.settings.general.colorMode.dark")}
          </SelectItem>
          <SelectItem
            value="darkPlus"
            className="focus:bg-black dark:focus:bg-black"
          >
            {t("pages.settings.general.colorMode.darkPlus")}
          </SelectItem>
          <SelectItem
            value="sky"
            className="focus:bg-sky-300 dark:focus:bg-sky-700"
          >
            {t("pages.settings.general.colorMode.sky")}
          </SelectItem>
          <SelectItem
            value="grove"
            className="focus:bg-emerald-300 dark:focus:bg-emerald-700"
          >
            {t("pages.settings.general.colorMode.grove")}
          </SelectItem>
          <SelectItem
            value="sunset"
            className="focus:bg-orange-400 dark:focus:bg-orange-700"
          >
            {t("pages.settings.general.colorMode.sunset")}
          </SelectItem>
          <SelectItem
            value="nebula"
            className="focus:bg-purple-300 dark:focus:bg-purple-900"
          >
            {t("pages.settings.general.colorMode.nebula")}
          </SelectItem>
          <SelectItem
            value="orbit"
            className="focus:bg-slate-300 dark:focus:bg-slate-500"
          >
            {t("pages.settings.general.colorMode.orbit")}
          </SelectItem>
          <SelectItem
            value="cappuccino"
            className="focus:bg-amber-300 dark:focus:bg-amber-500"
          >
            {t("pages.settings.general.colorMode.cappuccino")}
          </SelectItem>
          <SelectItem
            value="espresso"
            className="focus:bg-amber-500 dark:focus:bg-amber-800"
          >
            {t("pages.settings.general.colorMode.espresso")}
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
};
