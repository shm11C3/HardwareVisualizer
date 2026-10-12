import { useTranslation } from "react-i18next";
import { SettingWriteFailure } from "@/components/SettingWriteFailure";
import { Button } from "@/components/ui/button";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";

export const GraphColorReset = () => {
  const { resetLineGraphColors } = useSettingsAtom();
  const { t } = useTranslation();

  return (
    <>
      <Button
        onClick={() => void resetLineGraphColors()}
        className="mt-4"
        variant="secondary"
        size="lg"
      >
        {t("shared.reset")}
      </Button>
      <SettingWriteFailure actions="lineGraphColor.reset" />
    </>
  );
};
