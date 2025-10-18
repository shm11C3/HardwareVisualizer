import { useTranslation } from "react-i18next";
import { BurnInShift } from "@/components/shared/BurnInShift";
import { PreviewChart } from "@/features/settings/components/Preview";
import { BackgroundImageList } from "@/features/settings/components/SelectBackgroundImage";
import { UploadImage } from "@/features/settings/components/UploadImage";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { BackgroundOpacitySlider } from "./BackgroundOpacitySlider";
import { GraphColorSettings } from "./GraphColorSettings";
import { GraphStyleSettings } from "./GraphStyleSettings";
import { GraphTypeSelector } from "./GraphTypeSelector";

export const GraphSettings = () => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();

  return (
    <div className="mt-8 p-4">
      <h3 className="py-3 font-bold text-2xl">
        {t("pages.settings.customTheme.name")}
      </h3>
      <div className="items-start gap-x-12 gap-y-4 p-4 xl:grid xl:grid-cols-6">
        <div className="col-span-2 py-2">
          <h4 className="font-bold text-xl">
            {t("pages.settings.customTheme.graphStyle.name")}
          </h4>
          <GraphStyleSettings />
        </div>
        <div className="col-span-1 py-2">
          <GraphColorSettings />
          <div className="py-6">
            <h4 className="font-bold text-xl">
              {t("pages.settings.general.hardwareType")}
            </h4>
            <GraphTypeSelector />
          </div>
        </div>
        <div className="col-span-3 ml-10 py-2">
          <h4 className="font-bold text-xl">
            {t("pages.settings.customTheme.preview")}
          </h4>
          <BurnInShift enabled={settings.burnInShift}>
            <PreviewChart />
          </BurnInShift>
        </div>
        <div className="order-2 col-span-3 xl:order-none">
          <h4 className="py-3 font-bold text-xl">
            {t("pages.settings.backgroundImage.name")}
          </h4>
          <div className="p-1">
            <div className="py-3">
              <UploadImage />
              <BackgroundImageList />
            </div>
            <BackgroundOpacitySlider />
          </div>
        </div>
      </div>
    </div>
  );
};
