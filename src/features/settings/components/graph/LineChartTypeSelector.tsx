import { useTranslation } from "react-i18next";
import { LineChartIcon } from "@/components/icons/LineChartIcon";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { ClientSettings, LineGraphType } from "@/rspc/bindings";

export const LineChartTypeSelector = () => {
  const { settings, updateSettingAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const changeLineGraphType = async (
    value: ClientSettings["lineGraphType"],
  ) => {
    await updateSettingAtom("lineGraphType", value);
  };

  const lineGraphTypes: LineGraphType[] = [
    "default",
    "step",
    "linear",
    "basis",
  ] as const;

  const lineGraphLabels: Record<LineGraphType, string> = {
    default: "Default",
    step: "Step",
    linear: "Linear",
    basis: "Soft",
  };

  return (
    <div className="py-6">
      <Label htmlFor="lineGraphType" className="mb-2 text-lg">
        {t("pages.settings.customTheme.graphStyle.lineType")}
      </Label>
      <RadioGroup
        className="mt-4 flex items-center space-x-4"
        defaultValue={settings.lineGraphType}
        onValueChange={changeLineGraphType}
      >
        {lineGraphTypes.map((type) => {
          const id = `radio-line-chart-type-${type}`;

          return (
            <div key={type} className="flex items-center space-x-2">
              <RadioGroupItem value={type} id={id} />
              <Label
                className="flex items-center space-x-1 py-2 text-lg"
                htmlFor={id}
              >
                <LineChartIcon type={type} />
                <span>{lineGraphLabels[type]}</span>
              </Label>
            </div>
          );
        })}
      </RadioGroup>
    </div>
  );
};
