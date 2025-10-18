import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import type { ClientSettings } from "@/rspc/bindings";

export const BurnInShiftIdleCheckbox = ({
  settings,
  toggleIdleOnly,
}: {
  settings: ClientSettings;
  toggleIdleOnly: (value: boolean) => Promise<void>;
}) => {
  const { t } = useTranslation();
  const burnInShiftIdleOnlyId = useId();

  return (
    <div className="flex items-center space-x-2 py-4">
      <Checkbox
        id={burnInShiftIdleOnlyId}
        checked={settings.burnInShiftIdleOnly}
        onCheckedChange={toggleIdleOnly}
      />
      <Label
        htmlFor={burnInShiftIdleOnlyId}
        className="flex items-center space-x-2 text-lg"
      >
        {t("pages.settings.general.burnInShift.idleOnly.name")}
      </Label>
    </div>
  );
};
