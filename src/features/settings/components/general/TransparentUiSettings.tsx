import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

const minWindowOpacity = 20;
const maxWindowOpacity = 100;
const minGlassBlur = 0;
const maxGlassBlur = 30;

export const TransparentUiSettings = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const transparentUiId = useId();
  const windowOpacityId = useId();
  const windowOpacityLabelId = `${windowOpacityId}-label`;
  const glassBlurId = useId();
  const glassBlurLabelId = `${glassBlurId}-label`;

  const changeWindowOpacity = async (value: number[]) => {
    await updateSettingAtom("windowOpacity", value[0]);
  };

  const changeGlassBlur = async (value: number[]) => {
    await updateSettingAtom("glassBlur", value[0]);
  };

  return (
    <>
      <div className="flex w-full items-center justify-between gap-4 py-6 xl:w-1/2">
        <div className="space-y-1">
          <Label htmlFor={transparentUiId} className="text-lg">
            {t("pages.settings.general.transparentUi.name")}
          </Label>
          <p className="text-muted-foreground text-sm">
            {t("pages.settings.general.transparentUi.description")}
          </p>
        </div>

        <Switch
          id={transparentUiId}
          checked={settings.transparentUi}
          onCheckedChange={(value) => updateSettingAtom("transparentUi", value)}
        />
      </div>

      {settings.transparentUi && (
        <div className="grid w-full gap-6 py-3 xl:w-1/3">
          <div>
            <div className="mb-3 flex items-center justify-between">
              <Label id={windowOpacityLabelId} className="text-lg">
                {t("pages.settings.general.transparentUi.opacity")}
              </Label>
              <span className="font-medium text-muted-foreground text-sm tabular-nums">
                {settings.windowOpacity}%
              </span>
            </div>
            <Slider
              id={windowOpacityId}
              aria-labelledby={windowOpacityLabelId}
              min={minWindowOpacity}
              max={maxWindowOpacity}
              step={1}
              value={[settings.windowOpacity]}
              onValueChange={changeWindowOpacity}
              className="mt-4 w-full"
            />
          </div>

          <div>
            <div className="mb-3 flex items-center justify-between">
              <Label id={glassBlurLabelId} className="text-lg">
                {t("pages.settings.general.transparentUi.blur")}
              </Label>
              <span className="font-medium text-muted-foreground text-sm tabular-nums">
                {settings.glassBlur}px
              </span>
            </div>
            <Slider
              id={glassBlurId}
              aria-labelledby={glassBlurLabelId}
              min={minGlassBlur}
              max={maxGlassBlur}
              step={1}
              value={[settings.glassBlur]}
              onValueChange={changeGlassBlur}
              className="mt-4 w-full"
            />
          </div>
        </div>
      )}
    </>
  );
};
