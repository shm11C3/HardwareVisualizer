import { useId } from "react";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import type { ChartDataType } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";

export const GraphTypeSelector = () => {
  const { settings, toggleDisplayTarget } = useSettingsAtom();
  const selectedGraphTypes = settings.displayTargets;

  const cpuId = useId();
  const memoryId = useId();
  const gpuId = useId();

  const toggleGraphType = async (type: ChartDataType) => {
    await toggleDisplayTarget(type);
  };

  return (
    <div className="py-6">
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id={cpuId}
          checked={selectedGraphTypes.includes("cpu")}
          onCheckedChange={() => toggleGraphType("cpu")}
        />
        <Label htmlFor={cpuId} className="flex items-center space-x-2 text-lg">
          CPU
        </Label>
      </div>
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id={memoryId}
          checked={selectedGraphTypes.includes("memory")}
          onCheckedChange={() => toggleGraphType("memory")}
        />
        <Label
          htmlFor={memoryId}
          className="flex items-center space-x-2 text-lg"
        >
          RAM
        </Label>
      </div>
      <div className="flex items-center space-x-2 py-3">
        <Checkbox
          id={gpuId}
          checked={selectedGraphTypes.includes("gpu")}
          onCheckedChange={() => toggleGraphType("gpu")}
        />
        <Label htmlFor={gpuId} className="flex items-center space-x-2 text-lg">
          GPU
        </Label>
      </div>
    </div>
  );
};
