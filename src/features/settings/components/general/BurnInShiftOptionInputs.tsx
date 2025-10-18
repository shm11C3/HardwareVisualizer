import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { BurnInShiftOptions } from "@/rspc/bindings";

export const BurnInShiftOptionInputs = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();

  const intervalMsId = useId();
  const amplitudePxIdX = useId();
  const amplitudePxIdY = useId();
  const idleThresholdMsId = useId();
  const driftDurationSecId = useId();

  const inputVariants = "grid w-full max-w-3xs items-center gap-3";

  const updateBurnInShiftOption = async (
    field: keyof BurnInShiftOptions,
    value: number | [number, number] | null,
  ) => {
    const currentOptions = settings.burnInShiftOptions || {};
    const updatedOptions = { ...currentOptions, [field]: value };
    await updateSettingAtom(
      "burnInShiftOptions",
      updatedOptions as BurnInShiftOptions,
    );
  };

  const handleInputChange = (
    field: keyof BurnInShiftOptions,
    inputValue: string,
  ) => {
    if (inputValue === "") {
      updateBurnInShiftOption(field, null);
      return;
    }

    const numValue = Number(inputValue);
    if (!Number.isNaN(numValue)) {
      updateBurnInShiftOption(field, numValue);
    }
  };

  const handleAmplitudeChange = (axis: 0 | 1, inputValue: string) => {
    const currentAmplitude = settings.burnInShiftOptions?.amplitudePx || [0, 0];

    if (inputValue === "") {
      updateBurnInShiftOption("amplitudePx", null);
      return;
    }

    const numValue = Number(inputValue);
    if (!Number.isNaN(numValue)) {
      const newAmplitude: [number, number] = [...currentAmplitude];
      newAmplitude[axis] = numValue;
      updateBurnInShiftOption("amplitudePx", newAmplitude);
    }
  };

  return (
    <div className="flex flex-col space-y-4 py-4">
      <div className={inputVariants}>
        <Label htmlFor={intervalMsId}>
          {t("pages.settings.general.burnInShift.override.options.intervalMs")}
        </Label>
        <Input
          type="number"
          id={intervalMsId}
          placeholder="Not set"
          value={settings.burnInShiftOptions?.intervalMs ?? ""}
          onChange={(e) => handleInputChange("intervalMs", e.target.value)}
        />
      </div>
      <div className="flex space-x-4">
        <div className={inputVariants}>
          <Label htmlFor={amplitudePxIdX}>
            {t(
              "pages.settings.general.burnInShift.override.options.amplitudePxX",
            )}
          </Label>
          <Input
            type="number"
            id={amplitudePxIdX}
            placeholder="Not set"
            value={settings.burnInShiftOptions?.amplitudePx?.[0] ?? ""}
            onChange={(e) => handleAmplitudeChange(0, e.target.value)}
          />
        </div>

        <div className={inputVariants}>
          <Label htmlFor={amplitudePxIdY}>
            {t(
              "pages.settings.general.burnInShift.override.options.amplitudePxY",
            )}
          </Label>
          <Input
            type="number"
            id={amplitudePxIdY}
            placeholder="Not set"
            value={settings.burnInShiftOptions?.amplitudePx?.[1] ?? ""}
            onChange={(e) => handleAmplitudeChange(1, e.target.value)}
          />
        </div>
      </div>
      <div className={inputVariants}>
        <Label htmlFor={idleThresholdMsId}>
          {t(
            "pages.settings.general.burnInShift.override.options.idleThresholdMs",
          )}
        </Label>
        <Input
          type="number"
          id={idleThresholdMsId}
          placeholder="Not set"
          value={settings.burnInShiftOptions?.idleThresholdMs ?? ""}
          onChange={(e) => handleInputChange("idleThresholdMs", e.target.value)}
        />
      </div>
      <div className={inputVariants}>
        <Label htmlFor={driftDurationSecId}>
          {t(
            "pages.settings.general.burnInShift.override.options.driftDurationSec",
          )}
        </Label>
        <Input
          type="number"
          id={driftDurationSecId}
          placeholder="Not set"
          value={settings.burnInShiftOptions?.driftDurationSec ?? ""}
          onChange={(e) =>
            handleInputChange("driftDurationSec", e.target.value)
          }
        />
      </div>
    </div>
  );
};
