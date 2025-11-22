import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import type { BurnInShiftMode, ClientSettings } from "@/rspc/bindings";

export const BurnInShiftModeRadio = ({
  settings,
  selectShiftMode,
}: {
  settings: ClientSettings;
  selectShiftMode: (value: BurnInShiftMode) => Promise<void>;
}) => {
  const { t } = useTranslation();
  const radioBurnInShiftJump = useId();
  const radioBurnInShiftDrift = useId();
  return (
    <>
      <Label className="text-lg">
        {t("pages.settings.general.burnInShift.mode.name")}
      </Label>
      <RadioGroup
        className="mt-2 flex space-x-2"
        defaultValue={settings.burnInShiftMode}
        onValueChange={selectShiftMode}
      >
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="jump" id={radioBurnInShiftJump} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftJump}
          >
            <span>{t("pages.settings.general.burnInShift.mode.jump")}</span>
          </Label>
        </div>
        <div className="flex items-center space-x-2">
          <RadioGroupItem value="drift" id={radioBurnInShiftDrift} />
          <Label
            className="flex items-center space-x-2 text-md"
            htmlFor={radioBurnInShiftDrift}
          >
            <span>{t("pages.settings.general.burnInShift.mode.drift")}</span>
          </Label>
        </div>
      </RadioGroup>
    </>
  );
};
