import { useTranslation } from "react-i18next";
import { LoadFailure } from "@/components/LoadFailure";
import { Button } from "@/components/ui/button";
import {
  processGpuCoverage,
  processStatsWindowStart,
} from "@/features/hardware/insights/process/funcs/processGpuUsage";
import type { ProcessGpuRead } from "@/features/hardware/insights/types/processStats";
import {
  useDatabaseConversionSettings,
  useOffersDatabaseConversionSettings,
} from "@/hooks/navigation/useDatabaseConversionSettings";

/**
 * The one line above the Process Insight table that explains the GPU column's
 * state: its own read failure, the native database it needs (ADR 0030
 * decision 2 and 4), or a period that reaches back before GPU usage was first
 * recorded. `read` must answer the shown stats' window; it is null while that
 * read is pending, and the line then renders nothing, as it does when the GPU
 * column covers the whole period.
 */
export const ProcessGpuNotice = ({
  read,
  onRetry,
}: {
  read: ProcessGpuRead | null;
  onRetry: () => void;
}) => {
  const { t } = useTranslation();

  if (read == null) {
    return null;
  }

  if (read.status === "failed") {
    return (
      <LoadFailure
        className="mb-2 flex-row flex-wrap py-2"
        message={t("pages.insights.process.gpu.loadFailed")}
        onRetry={onRetry}
      />
    );
  }

  const processGpuStats = read.stats;
  if (processGpuStats.kind === "needsNativeDatabase") {
    return <NeedsNativeDatabaseNotice />;
  }

  const coverage = processGpuCoverage(
    processGpuStats.recorded_since,
    processStatsWindowStart(read.window),
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

// Its own component so conversion support is read only while this state shows.
const NeedsNativeDatabaseNotice = () => {
  const { t } = useTranslation();
  const openConversionSettings = useDatabaseConversionSettings();
  const offersConversion = useOffersDatabaseConversionSettings();

  return (
    <div
      className="mb-2 flex flex-wrap items-center gap-x-3 text-sm text-muted-foreground"
      data-testid="process-gpu-needs-native-database"
    >
      <span>{t("pages.insights.process.gpu.needsNativeDatabase")}</span>
      {offersConversion && (
        <Button
          type="button"
          variant="link"
          className="h-auto p-0"
          onClick={openConversionSettings}
        >
          {t("pages.insights.process.gpu.openSettings")}
        </Button>
      )}
    </div>
  );
};
