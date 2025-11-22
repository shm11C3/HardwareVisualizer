import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AboutSection } from "@/features/settings/components/about/AboutSection";
import { GeneralSettings } from "@/features/settings/components/general/GeneralSettings";
import { GraphSettings } from "@/features/settings/components/graph/GraphSettings";
import { InsightsSettings } from "@/features/settings/components/insights/InsightsSettings";
import { LicensePage } from "@/features/settings/components/LicensePage";

export const Settings = () => {
  const { t } = useTranslation();
  const [showLicensePage, setShowLicensePage] = useState(false);

  if (showLicensePage) {
    return <LicensePage onBack={() => setShowLicensePage(false)} />;
  }

  return (
    <>
      <GeneralSettings />
      <GraphSettings />
      <InsightsSettings />

      <div className="p-4">
        <h3 className="py-3 font-bold text-2xl">
          {t("pages.settings.about.name")}
        </h3>
        <AboutSection onShowLicense={() => setShowLicensePage(true)} />
      </div>
    </>
  );
};
