import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { BurnInShiftMode, BurnInShiftPreset } from "@/rspc/bindings";
import { BurnInShiftIdleCheckbox } from "./BurnInShiftIdleCheckbox";
import { BurnInShiftModeRadio } from "./BurnInShiftModeRadio";
import { BurnInShiftOptionInputs } from "./BurnInShiftOptionInputs";
import { BurnInShiftOverrideCheckbox } from "./BurnInShiftOverrideCheckbox";
import { BurnInShiftPresetRadio } from "./BurnInShiftPresetRadio";

export const BurnInShiftSettings = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();

  const [defaultOpen, setDefaultOpen] = useState(false);

  const toggleBurnInShiftId = useId();

  const toggleBurnInShift = async (value: boolean) => {
    setDefaultOpen(value);
    await updateSettingAtom("burnInShift", value);
  };
  const selectShiftPreset = async (value: BurnInShiftPreset) => {
    await updateSettingAtom("burnInShiftPreset", value);
  };
  const selectShiftMode = async (value: BurnInShiftMode) => {
    await updateSettingAtom("burnInShiftMode", value);
  };
  const toggleIdleOnly = async (value: boolean) => {
    await updateSettingAtom("burnInShiftIdleOnly", value);
  };

  return (
    <>
      <div className="flex w-full items-center justify-between space-x-4 py-6 xl:w-1/3">
        <div className="space-y-0.5">
          <Label htmlFor={toggleBurnInShiftId} className="text-lg">
            {t("pages.settings.general.burnInShift.name")}
          </Label>
        </div>

        <Switch
          id={toggleBurnInShiftId}
          checked={settings.burnInShift}
          onCheckedChange={toggleBurnInShift}
        />
      </div>
      {settings.burnInShift && (
        <Accordion
          type="single"
          collapsible
          className="w-full xl:w-1/3"
          defaultValue={defaultOpen ? "burnInShiftSettings" : undefined}
        >
          <AccordionItem value="burnInShiftSettings">
            <AccordionTrigger>
              {t("pages.settings.general.burnInShift.detailSettings")}
            </AccordionTrigger>
            <AccordionContent>
              <div className="flex flex-col space-y-2">
                <div className="py-4">
                  <BurnInShiftPresetRadio
                    settings={settings}
                    selectShiftPreset={selectShiftPreset}
                  />
                </div>
                <div className="py-4">
                  <BurnInShiftModeRadio
                    settings={settings}
                    selectShiftMode={selectShiftMode}
                  />
                </div>

                <BurnInShiftIdleCheckbox
                  settings={settings}
                  toggleIdleOnly={toggleIdleOnly}
                />

                <BurnInShiftOverrideCheckbox />
                {settings.burnInShiftOptions !== null && (
                  <BurnInShiftOptionInputs />
                )}
              </div>
            </AccordionContent>
          </AccordionItem>
        </Accordion>
      )}
    </>
  );
};
