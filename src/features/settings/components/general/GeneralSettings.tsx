import { useTranslation } from "react-i18next";
import { AutoStartToggle } from "./AutoStartToggle";
import { BurnInShiftSettings } from "./BurnInShiftSettings";
import { LanguageSelect } from "./LanguageSelect";
import { TemperatureUnitSelect } from "./TemperatureUnitSelect";
import { ThemeSelect } from "./ThemeSelect";

export const GeneralSettings = () => {
  const { t } = useTranslation();

  return (
    <div className="mt-8 p-4">
      <h3 className="py-3 font-bold text-2xl">
        {t("pages.settings.general.name")}
      </h3>
      <div className="px-4">
        <LanguageSelect />
        <ThemeSelect />
        <TemperatureUnitSelect />
        <AutoStartToggle />
        <BurnInShiftSettings />
      </div>
    </div>
  );
};
