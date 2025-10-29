import { useId } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import { Switch } from "@/components/ui/switch";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { BurnInShiftOptions, PanelAspect } from "@/rspc/bindings";

export const BurnInShiftOptionInputs = () => {
  const { t } = useTranslation();
  const { settings, updateSettingAtom } = useSettingsAtom();

  const intervalMsId = useId();
  const amplitudePxIdX = useId();
  const amplitudePxIdY = useId();
  const idleThresholdMsId = useId();
  const driftDurationSecId = useId();
  const panelScaleId = useId();
  const panelAspectId = useId();
  const roamAreaPercentId = useId();
  const keepWithinBoundsId = useId();

  const inputVariants = "grid w-full max-w-3xs items-center gap-3";

  const updateBurnInShiftOption = async (
    field: keyof BurnInShiftOptions,
    value: number | [number, number] | boolean | PanelAspect | null,
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

      {/* Panel Scale */}
      <div className={inputVariants}>
        <Label htmlFor={panelScaleId}>
          {t("pages.settings.general.burnInShift.override.options.panelScale")}
        </Label>
        <div className="flex items-center gap-4">
          <Slider
            id={panelScaleId}
            min={50}
            max={150}
            step={5}
            value={[settings.burnInShiftOptions?.panelScale ?? 100]}
            onValueChange={(value) =>
              updateBurnInShiftOption("panelScale", value[0])
            }
            className="flex-1"
          />
          <span className="w-12 text-center">
            {settings.burnInShiftOptions?.panelScale ?? 100}%
          </span>
        </div>
      </div>

      {/* Panel Aspect */}
      <div className={inputVariants}>
        <Label htmlFor={panelAspectId}>
          {t("pages.settings.general.burnInShift.override.options.panelAspect")}
        </Label>
        <Select
          value={settings.burnInShiftOptions?.panelAspect ?? "auto"}
          onValueChange={(value) =>
            updateBurnInShiftOption("panelAspect", value as PanelAspect)
          }
        >
          <SelectTrigger id={panelAspectId}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="auto">
              {t("pages.settings.general.burnInShift.panelAspect.auto")}
            </SelectItem>
            <SelectItem value="compact">
              {t("pages.settings.general.burnInShift.panelAspect.compact")}
            </SelectItem>
            <SelectItem value="tall">
              {t("pages.settings.general.burnInShift.panelAspect.tall")}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Roam Area */}
      <div className={inputVariants}>
        <Label htmlFor={roamAreaPercentId}>
          {t(
            "pages.settings.general.burnInShift.override.options.roamAreaPercent",
          )}
        </Label>
        <div className="flex items-center gap-4">
          <Slider
            id={roamAreaPercentId}
            min={50}
            max={100}
            step={5}
            value={[settings.burnInShiftOptions?.roamAreaPercent ?? 100]}
            onValueChange={(value) =>
              updateBurnInShiftOption("roamAreaPercent", value[0])
            }
            className="flex-1"
          />
          <span className="w-12 text-center">
            {settings.burnInShiftOptions?.roamAreaPercent ?? 100}%
          </span>
        </div>
      </div>

      {/* Keep Within Bounds */}
      <div className="flex w-full items-center justify-between space-x-4 py-2">
        <Label htmlFor={keepWithinBoundsId}>
          {t(
            "pages.settings.general.burnInShift.override.options.keepWithinBounds",
          )}
        </Label>
        <Switch
          id={keepWithinBoundsId}
          checked={settings.burnInShiftOptions?.keepWithinBounds ?? true}
          onCheckedChange={(value) =>
            updateBurnInShiftOption("keepWithinBounds", value)
          }
        />
      </div>
    </div>
  );
};
