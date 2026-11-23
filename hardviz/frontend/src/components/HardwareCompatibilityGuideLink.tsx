import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { Button } from "@/components/ui/button";
import { openURL } from "@/lib/openUrl";

export const HardwareCompatibilityGuideLink = () => {
  const { t } = useTranslation();
  const [failed, setFailed] = useState(false);

  const open = async () => {
    setFailed(false);
    try {
      await openURL(
        "https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/hardware-compatibility.md",
      );
    } catch {
      setFailed(true);
    }
  };

  return (
    <div>
      <Button
        type="button"
        variant="link"
        className="h-auto p-0 text-left whitespace-normal"
        onClick={() => void open()}
      >
        {t("hardwareCompatibilityReport.guide")}
      </Button>
      {failed && (
        <ActionFailure
          message={t("hardwareCompatibilityReport.errors.openGuide")}
        />
      )}
    </div>
  );
};
