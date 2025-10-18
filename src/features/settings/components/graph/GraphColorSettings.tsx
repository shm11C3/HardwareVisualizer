import { useTranslation } from "react-i18next";
import { GraphColorPicker } from "./GraphColorPicker";
import { GraphColorReset } from "./GraphColorReset";

export const GraphColorSettings = () => {
  const { t } = useTranslation();

  return (
    <div className="py-6">
      <h4 className="font-bold text-xl">
        {t("pages.settings.customTheme.lineColor")}
      </h4>
      <div className="py-6 md:flex lg:block">
        <GraphColorPicker label="CPU" hardwareType="cpu" />
        <GraphColorPicker label="RAM" hardwareType="memory" />
        <GraphColorPicker label="GPU" hardwareType="gpu" />
      </div>
      <GraphColorReset />
    </div>
  );
};
