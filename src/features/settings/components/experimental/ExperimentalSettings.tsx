import { useTranslation } from "react-i18next";
import { TransparentUiSettings } from "@/features/settings/components/general/TransparentUiSettings";

export const ExperimentalSettings = () => {
  const { t } = useTranslation();

  return (
    <div className="mt-8 p-4">
      <h3 className="py-3 font-bold text-2xl">
        {t("pages.settings.experimental.name")}
      </h3>
      <p className="px-4 text-muted-foreground text-sm">
        {t("pages.settings.experimental.description")}
      </p>
      <div className="px-4">
        <TransparentUiSettings />
      </div>
    </div>
  );
};
