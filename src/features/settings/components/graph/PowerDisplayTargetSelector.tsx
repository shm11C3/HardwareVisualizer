import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { PowerDisplayTarget } from "@/rspc/bindings";

const targets: PowerDisplayTarget[] = ["cpu", "gpu", "ane", "package"];

export const PowerDisplayTargetSelector = () => {
  const { t } = useTranslation();
  const { settings, togglePowerDisplayTarget } = useSettingsAtom();
  const baseId = useId();

  return (
    <div className="py-6">
      {targets.map((target) => {
        const id = `${baseId}-${target}`;
        return (
          <div key={target} className="flex items-center space-x-2 py-3">
            <Checkbox
              id={id}
              checked={settings.powerDisplayTargets.includes(target)}
              onCheckedChange={() => togglePowerDisplayTarget(target)}
            />
            <Label htmlFor={id} className="text-lg">
              {t(`pages.performance.power.${target}`)}
            </Label>
          </div>
        );
      })}
    </div>
  );
};
