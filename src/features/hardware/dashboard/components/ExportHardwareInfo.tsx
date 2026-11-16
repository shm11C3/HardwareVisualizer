import { CheckIcon, CopyIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { HardwareCompatibilityReport } from "@/components/HardwareCompatibilityReport";
import { Button } from "@/components/ui/button";
import { useCopyFeedback } from "@/features/hardware/dashboard/hooks/useCopyFeedback";
import { useExportToClipboard } from "@/features/hardware/dashboard/hooks/useExportToClipboard";

/** Dashboard header actions: icon-only copy and compatibility report. */
export const ExportHardwareInfo = () => {
  const { exportToClipboard } = useExportToClipboard();
  const { copied, failed, copy } = useCopyFeedback(exportToClipboard);
  const { t } = useTranslation();
  const label = t("pages.dashboard.copyHardwareReport");

  return (
    <>
      <div className="space-y-1">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          aria-label={label}
          title={label}
          onClick={() => void copy()}
        >
          {copied ? (
            <CheckIcon className="size-5" aria-hidden="true" />
          ) : (
            <CopyIcon className="size-5" aria-hidden="true" />
          )}
        </Button>
        {failed && <ActionFailure message={t("pages.dashboard.copyFailed")} />}
      </div>
      <HardwareCompatibilityReport icon />
    </>
  );
};
