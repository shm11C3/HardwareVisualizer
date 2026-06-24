import { useTranslation } from "react-i18next";
import { AutoStartToggle } from "./AutoStartToggle";
import { BurnInShiftSettings } from "./BurnInShiftSettings";
import { CloseToTrayToggle } from "./CloseToTrayToggle";
import { LanguageSelect } from "./LanguageSelect";
import { TemperatureUnitSelect } from "./TemperatureUnitSelect";
import { TextSelectionToggle } from "./TextSelectionToggle";
import { ThemeSelect } from "./ThemeSelect";
import { TransparentUiSettings } from "./TransparentUiSettings";
import { TrayWidgetSettings } from "./TrayWidgetSettings";

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
        <TransparentUiSettings />
        <TemperatureUnitSelect />
        <CloseToTrayToggle />
        <TrayWidgetSettings />
        <AutoStartToggle />
        <TextSelectionToggle />
        <BurnInShiftSettings />
      </div>
    </div>
  );
};
