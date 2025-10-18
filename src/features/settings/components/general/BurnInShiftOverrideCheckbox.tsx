import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { BurnInShiftOptions } from "@/rspc/bindings";

export const BurnInShiftOverrideCheckbox = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();
  const id = useId();

  const handleOverrideChange = (checked: boolean) => {
    updateSettingAtom(
      "burnInShiftOptions",
      checked ? ({} as BurnInShiftOptions) : null,
    );
  };

  return (
    <div className="flex items-center space-x-2 py-4">
      <Checkbox
        id={id}
        checked={settings.burnInShiftOptions !== null}
        onCheckedChange={handleOverrideChange}
      />
      <Label htmlFor={id} className="flex items-center space-x-2 text-lg">
        {t("pages.settings.general.burnInShift.override.name")}
      </Label>
    </div>
  );
};
