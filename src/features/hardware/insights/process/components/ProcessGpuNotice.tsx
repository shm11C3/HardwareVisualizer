import { useTranslation } from "react-i18next";
import { LoadFailure } from "@/components/LoadFailure";
import { Button } from "@/components/ui/button";
import { processGpuCoverage } from "@/features/hardware/insights/process/funcs/processGpuUsage";
import { useDatabaseConversionSettings } from "@/hooks/navigation/useDatabaseConversionSettings";
import type { ProcessGpuStats } from "@/rspc/bindings";

/**
 * The one line above the Process Insight table that explains the GPU column's
 * state: its own read failure, the native database it needs (ADR 0030
 * decision 2 and 4), or a period that reaches back before GPU usage was first
 * recorded. Renders nothing while the first read is pending or when the GPU
 * column covers the whole period.
 */
export const ProcessGpuNotice = ({
  processGpuStats,
  hasError,
  onRetry,
  windowStart,
}: {
  processGpuStats: ProcessGpuStats | null;
  hasError: boolean;
  onRetry: () => void;
  windowStart: Date;
}) => {
  const { t } = useTranslation();
  const openConversionSettings = useDatabaseConversionSettings();

  if (hasError) {
    return (
      <LoadFailure
        className="mb-2 flex-row flex-wrap py-2"
        message={t("pages.insights.process.gpu.loadFailed")}
        onRetry={onRetry}
      />
    );
  }

  if (processGpuStats == null) {
    return null;
  }

  if (processGpuStats.kind === "needsNativeDatabase") {
    return (
      <div
        className="mb-2 flex flex-wrap items-center gap-x-3 text-sm text-muted-foreground"
        data-testid="process-gpu-needs-native-database"
      >
        <span>{t("pages.insights.process.gpu.needsNativeDatabase")}</span>
        <Button
          type="button"
          variant="link"
          className="h-auto p-0"
          onClick={openConversionSettings}
        >
          {t("pages.insights.process.gpu.openSettings")}
        </Button>
      </div>
    );
  }

  const coverage = processGpuCoverage(
    processGpuStats.recorded_since,
    windowStart,
  );
  if (coverage.kind === "complete") {
    return null;
  }

  return (
    <p
      className="mb-2 text-xs text-muted-foreground"
      data-testid="process-gpu-coverage-note"
    >
      {coverage.kind === "recordedSince"
        ? t("pages.insights.process.gpu.recordedSince", {
            date: coverage.recordedSince.toLocaleString(),
          })
        : t("pages.insights.process.gpu.notRecordedYet")}
    </p>
  );
};
