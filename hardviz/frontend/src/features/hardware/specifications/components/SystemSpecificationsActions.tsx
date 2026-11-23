import { CheckIcon, CopyIcon } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ActionFailure } from "@/components/ActionFailure";
import { HardwareCompatibilityReport } from "@/components/HardwareCompatibilityReport";
import { Button } from "@/components/ui/button";
import { useCopyFeedback } from "@/features/hardware/dashboard/hooks/useCopyFeedback";
import { useExportToClipboard } from "@/features/hardware/dashboard/hooks/useExportToClipboard";

/** System Specifications header actions: labeled copy and compatibility report. */
export const SystemSpecificationsActions = () => {
  const { exportToClipboard } = useExportToClipboard({
    includeRuntimeStats: false,
  });
  const { copied, failed, copy } = useCopyFeedback(exportToClipboard);
  const { t } = useTranslation();

  return (
    <div className="flex flex-wrap items-start gap-2">
      <div className="space-y-1">
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="gap-2"
          onClick={() => void copy()}
        >
          {copied ? (
            <CheckIcon className="size-4" aria-hidden="true" />
          ) : (
            <CopyIcon className="size-4" aria-hidden="true" />
          )}
          {copied
            ? t("pages.dashboard.copied")
            : t("pages.dashboard.systemSpecifications.copySpecs")}
        </Button>
        <span role="status" className="sr-only">
          {copied ? t("pages.dashboard.copied") : ""}
        </span>
        {failed && <ActionFailure message={t("pages.dashboard.copyFailed")} />}
      </div>
      <HardwareCompatibilityReport
        variant="outline"
        size="sm"
        label={t("hardwareCompatibilityReport.shortLabel")}
      />
    </div>
  );
};
