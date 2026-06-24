import { platform } from "@tauri-apps/plugin-os";
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
// On macOS glass_blur is a frost-intensity level mapped to a vibrancy material:
// 0 = off, 1-10 = low, 11-20 = medium, 21+ = high. The macOS control is a
// 4-step slider; macFrostStops is the glass_blur value stored per step.
type MacFrostLevel = "off" | "low" | "medium" | "high";
const macFrostStops = [0, 8, 16, 26];
const glassBlurToMacFrostIndex = (v: number): number => {
  if (v <= 0) return 0;
  if (v <= 10) return 1;
  if (v <= 20) return 2;
  return 3;
};
const glassBlurToMacFrostLevel = (v: number): MacFrostLevel => {
  if (v <= 0) return "off";
  if (v <= 10) return "low";
  if (v <= 20) return "medium";
  return "high";
};

export const TransparentUiSettings = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const isMacOS = platform() === "macos";
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

  const changeMacFrostLevel = async (value: number[]) => {
    await updateSettingAtom("glassBlur", macFrostStops[value[0] ?? 0] ?? 0);
  };

  const macFrostLevelLabel = (): string => {
    switch (glassBlurToMacFrostLevel(settings.glassBlur)) {
      case "off":
        return t("pages.settings.general.transparentUi.blurLevel.off");
      case "low":
        return t("pages.settings.general.transparentUi.blurLevel.low");
      case "medium":
        return t("pages.settings.general.transparentUi.blurLevel.medium");
      default:
        return t("pages.settings.general.transparentUi.blurLevel.high");
    }
  };

  return (
    <>
      <div className="flex w-full items-center justify-between gap-4 py-6 xl:w-1/2">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Label htmlFor={transparentUiId} className="text-lg">
              {t("pages.settings.general.transparentUi.name")}
            </Label>
            <span className="rounded-full bg-muted px-2 py-0.5 font-medium text-muted-foreground text-xs">
              {t("pages.settings.general.transparentUi.experimentalBadge")}
            </span>
          </div>
          <p className="text-muted-foreground text-sm">
            {t("pages.settings.general.transparentUi.description")}
          </p>
          <p className="text-muted-foreground text-xs">
            {t("pages.settings.general.transparentUi.experimentalNote")}
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
                {isMacOS ? macFrostLevelLabel() : `${settings.glassBlur}px`}
              </span>
            </div>
            {isMacOS ? (
              // Native vibrancy materials are discrete, so the macOS slider
              // snaps to 4 steps (off / low / medium / high).
              <Slider
                id={glassBlurId}
                aria-labelledby={glassBlurLabelId}
                min={0}
                max={3}
                step={1}
                value={[glassBlurToMacFrostIndex(settings.glassBlur)]}
                onValueChange={changeMacFrostLevel}
                className="mt-4 w-full"
              />
            ) : (
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
            )}
          </div>
        </div>
      )}
    </>
  );
};
