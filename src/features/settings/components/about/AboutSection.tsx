import { ArrowSquareOutIcon, GithubLogoIcon } from "@phosphor-icons/react";
import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { openURL } from "@/lib/openUrl";

export const AboutSection = ({
  onShowLicense,
}: {
  onShowLicense: () => void;
}) => {
  const { t } = useTranslation();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then((v) => setVersion(v));
  }, []);

  return (
    <div className="px-4 py-2">
      <p className="text-gray-500 text-sm">
        {t("pages.settings.about.version", { version })}
      </p>
      <p className="text-gray-500 text-sm">
        {t("pages.settings.about.author", { author: "shm11C3" })}
      </p>
      <div className="flex items-center space-x-4 py-4">
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
          onClick={onShowLicense}
          variant="secondary"
          className="rounded-full text-sm"
        >
          {t("pages.settings.about.license")}
        </Button>
      </div>
    </div>
  );
};
