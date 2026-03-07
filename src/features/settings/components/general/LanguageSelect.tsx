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

export const LanguageSelect = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t, i18n } = useTranslation();

  const supported = Object.keys(i18n.services.resourceStore.data).sort((a, b) =>
    a.localeCompare(b, "en"),
  );

  const displaySupported: Record<string, string> = supported.reduce(
    (acc, lang) => {
      const translated = t(`lang.${lang}` as never);
      const native = i18n.t(`lang.${lang}` as never, { lng: lang });
      acc[lang] =
        native && native !== translated
          ? `${translated} (${native})`
          : translated;
      return acc;
    },
    {} as Record<string, string>,
  );

  const changeLanguage = async (value: string) => {
    await updateSettingAtom("language", value);
  };

  return (
    <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
      <Label htmlFor="language" className="text-lg">
        {t("pages.settings.general.language")}
      </Label>
      <Select value={settings.language} onValueChange={changeLanguage}>
        <SelectTrigger className="w-[180px]">
          <SelectValue placeholder="Language" />
        </SelectTrigger>
        <SelectContent>
          {supported.map((lang) => (
            <SelectItem key={lang} value={lang}>
              {displaySupported[lang]}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </div>
  );
};
