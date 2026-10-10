import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { defaultColorRGB } from "@/consts/chart";
import { useSettingsAtom } from "@/hooks/useSettingsAtom";
import { RGB2HEX } from "@/lib/color";
import { chartHardwareTypes } from "@/types/chart";

export const GraphColorReset = () => {
  const { updateLineGraphColorAtom } = useSettingsAtom();
  const { t } = useTranslation();

  const updateGraphColor = async () => {
    await Promise.all(
      chartHardwareTypes.map((type) =>
        updateLineGraphColorAtom(type, RGB2HEX(defaultColorRGB[type])),
      ),
    );
  };

  return (
    <Button
      onClick={() => updateGraphColor()}
      className="mt-4"
      variant="secondary"
      size="lg"
    >
      {t("shared.reset")}
    </Button>
  );
};
