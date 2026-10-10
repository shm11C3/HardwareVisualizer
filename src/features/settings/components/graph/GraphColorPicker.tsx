import { Label } from "@/components/ui/label";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { RGB2HEX } from "@/lib/color";
import type { ChartDataType } from "@/types/chart";

export const GraphColorPicker = ({
  label,
  hardwareType,
}: {
  label: string;
  hardwareType: ChartDataType;
}) => {
  const { settings, updateLineGraphColorAtom } = useSettingsAtom();

  const updateGraphColor = async (value: string) => {
    await updateLineGraphColorAtom(hardwareType, value);
  };

  // Convert comma-separated RGB values to hexadecimal
  const hexValue = RGB2HEX(settings.lineGraphColor[hardwareType]);

  return (
    <div className="grid grid-cols-2 gap-4 py-3">
      <Label htmlFor={hardwareType} className="text-lg">
        {label}
      </Label>

      <input
        type="color"
        className="h-6 w-6 cursor-pointer border-none p-0"
        value={hexValue}
        onChange={(e) => updateGraphColor(e.target.value)}
      />
    </div>
  );
};
