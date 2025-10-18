import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

export const BackgroundOpacitySlider = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const changeBackGroundOpacity = async (value: number[]) => {
    updateSettingAtom("backgroundImgOpacity", value[0]);
  };

  return (
    settings.selectedBackgroundImg && (
      <div className="max-w-96 py-3">
        <Label className="mb-2 block text-lg">
          {t("pages.settings.backgroundImage.opacity")}
        </Label>
        <Slider
          min={0}
          max={100}
          step={1}
          value={[settings.backgroundImgOpacity]}
          onValueChange={changeBackGroundOpacity}
          className="mt-4 w-full"
        />
      </div>
    )
  );
};
