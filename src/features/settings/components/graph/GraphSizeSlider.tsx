import { DotOutlineIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { sizeOptions } from "@/features/hardware/consts/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { Settings } from "@/features/settings/types/settingsType";

export const GraphSizeSlider = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const sizeIndex = sizeOptions.indexOf(
    settings.graphSize as Settings["graphSize"],
  );

  const changeGraphSize = async (value: number[]) => {
    await updateSettingAtom("graphSize", sizeOptions[value[0]]);
  };

  return (
    <div className="w-full py-6">
      <Label className="mb-2 block text-lg">
        {t("pages.settings.customTheme.graphStyle.size")}
      </Label>
      <Slider
        min={0}
        max={sizeOptions.length - 1}
        step={1}
        value={[sizeIndex]}
        onValueChange={changeGraphSize}
        className="mt-4 w-full"
      />
      <div className="mt-2 flex items-center justify-between text-sm">
        {sizeOptions.map((size) => (
          <DotOutlineIcon
            key={size}
            className="text-slate-600 dark:text-gray-400"
            size={32}
          />
        ))}
      </div>
    </div>
  );
};
