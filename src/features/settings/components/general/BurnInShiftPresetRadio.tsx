import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import type { BurnInShiftPreset, ClientSettings } from "@/rspc/bindings";

export const BurnInShiftPresetRadio = ({
  settings,
  selectShiftPreset,
}: {
  settings: ClientSettings;
  selectShiftPreset: (value: BurnInShiftPreset) => Promise<void>;
}) => {
  const { t } = useTranslation();
  const radioBurnInShiftGentle = useId();
  const radioBurnInShiftBalanced = useId();
  const radioBurnInShiftAggressive = useId();

  return (
    <>
      <Label className="text-lg">
        {t("pages.settings.general.burnInShift.preset.name")}
      </Label>
      <RadioGroup
        className="mt-2 flex space-x-2"
        defaultValue={settings.burnInShiftPreset}
        onValueChange={selectShiftPreset}
      >
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="gentle" id={radioBurnInShiftGentle} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftGentle}
          >
            <span>{t("pages.settings.general.burnInShift.preset.gentle")}</span>
          </Label>
        </div>
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="balanced" id={radioBurnInShiftBalanced} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftBalanced}
          >
            <span>
              {t("pages.settings.general.burnInShift.preset.balanced")}
            </span>
          </Label>
        </div>
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="aggressive" id={radioBurnInShiftAggressive} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftAggressive}
          >
            <span>
              {t("pages.settings.general.burnInShift.preset.aggressive")}
            </span>
          </Label>
        </div>
      </RadioGroup>
    </>
  );
};
