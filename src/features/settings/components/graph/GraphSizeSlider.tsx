import { DotOutlineIcon } from "@phosphor-icons/react";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { Slider } from "@/components/ui/slider";
import { sizeOptions } from "@/features/hardware/consts/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { Settings } from "@/features/settings/types/settingsType";
import { cn } from "@/lib/utils";
import { GraphMarginInput } from "./GraphMarginInput";

export const GraphSizeSlider = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();
  const fitToWindowId = useId();

  // The fixed size steps are meaningless while graphs follow the window size
  const disabled = settings.graphFitToWindow;

  const sizeIndex = sizeOptions.indexOf(
    settings.graphSize as Settings["graphSize"],
  );

  const changeGraphSize = async (value: number[]) => {
    await updateSettingAtom("graphSize", sizeOptions[value[0]]);
  };

  const changeFitToWindow = async (checked: boolean | "indeterminate") => {
    await updateSettingAtom("graphFitToWindow", checked === true);
  };

  return (
    <div className="w-full py-3">
      <fieldset className="w-full">
        <legend className="text-lg">
          {t("pages.settings.customTheme.graphStyle.size")}
        </legend>

        <div
          className="grid gap-x-8 sm:grid-cols-[minmax(0,1fr)_12rem]"
          data-testid="graph-size-row"
        >
          <div className={cn("pt-2", disabled && "opacity-50")}>
            <Slider
              min={0}
              max={sizeOptions.length - 1}
              step={1}
              value={[sizeIndex]}
              onValueChange={changeGraphSize}
              className="mt-4 w-full"
              disabled={disabled}
              aria-label={t("pages.settings.customTheme.graphStyle.size")}
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

          <GraphMarginInput />
        </div>

        <div className="flex items-center gap-3 py-2">
          <Checkbox
            id={fitToWindowId}
            checked={settings.graphFitToWindow}
            onCheckedChange={changeFitToWindow}
          />
          <Label htmlFor={fitToWindowId} className="text-base">
            {t("pages.settings.customTheme.graphStyle.graphFitToWindow")}
          </Label>
        </div>
      </fieldset>
    </div>
  );
};
