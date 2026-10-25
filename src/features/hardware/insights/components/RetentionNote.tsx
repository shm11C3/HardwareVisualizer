import { useTranslation } from "react-i18next";
import type { ArchivePeriod } from "@/features/hardware/insights/utils/archivePeriod";
import { periodExceedsRetention } from "@/features/hardware/insights/utils/retention";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";

/**
 * One line under the tab's period selector explaining why the oldest part of
 * a chart is empty: the selected range reaches past the Hardware Archive's
 * Retention Period. Shown only when some chart's period actually does.
 */
export const RetentionNote = ({
  periods,
}: {
  periods: readonly (ArchivePeriod | null)[];
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { hardwareArchive } = settings;

  const exceeds = periods.some(
    (period) =>
      period != null && periodExceedsRetention(period, hardwareArchive),
  );
  if (!exceeds) {
    return null;
  }

  return (
    <p
      className="mt-2 text-right text-muted-foreground text-xs"
      data-testid="insight-retention-note"
    >
      {t("pages.insights.retentionNote", {
        days: hardwareArchive.retentionDays,
      })}
    </p>
  );
};
