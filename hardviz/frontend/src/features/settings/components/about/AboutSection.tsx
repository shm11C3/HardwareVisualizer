import {
  ArrowSquareOutIcon,
  ClipboardTextIcon,
  GithubLogoIcon,
} from "@phosphor-icons/react";
import { getVersion } from "@tauri-apps/api/app";
import { platform } from "@tauri-apps/plugin-os";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { HardwareCompatibilityReport } from "@/components/HardwareCompatibilityReport";
import { Button } from "@/components/ui/button";
import { openURL } from "@/lib/openUrl";
import { surveyUrl } from "@/lib/survey";

export const AboutSection = ({
  onShowLicense,
}: {
  onShowLicense: () => void;
}) => {
  const { t, i18n } = useTranslation();
  const [version, setVersion] = useState("");
  const [surveyFailed, setSurveyFailed] = useState(false);

  useEffect(() => {
    getVersion().then((v) => setVersion(v));
  }, []);

  // Read the version at click time so an early click still carries it.
  const openSurvey = async () => {
    setSurveyFailed(false);
    try {
      await openURL(
        surveyUrl({
          language: i18n.resolvedLanguage ?? i18n.language,
          appVersion: await getVersion(),
          os: platform(),
        }),
      );
    } catch {
      setSurveyFailed(true);
    }
  };

  return (
    <div className="px-4 py-2">
      <p className="text-sm text-gray-500">
        {t("pages.settings.about.version", { version })}
      </p>
      <p className="text-sm text-gray-500">
        {t("pages.settings.about.author", { author: "shm11C3" })}
      </p>
      <div className="flex flex-wrap items-center gap-4 py-4">
        <Button
          onClick={() =>
            openURL("https://github.com/shm11C3/HardwareVisualizer")
          }
          variant="secondary"
          className="rounded-full text-sm"
        >
          <GithubLogoIcon size={32} />
          <span className="px-1">{t("pages.settings.about.checkGitHub")}</span>
          <ArrowSquareOutIcon size={16} />
        </Button>
        <Button
          onClick={() =>
            openURL(
              "https://github.com/shm11C3/HardwareVisualizer/releases/latest",
            )
          }
          variant="secondary"
          className="rounded-full text-sm"
        >
          <span className="px-1">
            {t("pages.settings.about.checkLatestVersion")}
          </span>
          <ArrowSquareOutIcon size={16} />
        </Button>
        <Button
          onClick={() => void openSurvey()}
          variant="secondary"
          className="rounded-full text-sm"
        >
          <ClipboardTextIcon size={20} />
          <span className="px-1">{t("pages.settings.about.answerSurvey")}</span>
          <ArrowSquareOutIcon size={16} />
        </Button>
        <Button
          onClick={onShowLicense}
          variant="secondary"
          className="rounded-full text-sm"
        >
          {t("pages.settings.about.license")}
        </Button>
      </div>
      {surveyFailed && (
        <ActionFailure
          message={t("pages.settings.about.openSurveyFailed")}
          className="pb-4"
        />
      )}
      <HardwareCompatibilityReport />
    </div>
  );
};
