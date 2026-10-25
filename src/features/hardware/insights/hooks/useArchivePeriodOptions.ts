import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { ArchivePeriod } from "@/features/hardware/insights/utils/archivePeriod";
import { describeArchivePeriod } from "@/features/hardware/insights/utils/archivePeriodLabel";

export type ArchivePeriodOption<P extends ArchivePeriod = ArchivePeriod> = {
  label: string;
  value: P;
};

/**
 * The period selector's options for one list of periods, labelled in the
 * largest whole unit each period fits ("30 minutes", "3 hours", "7 days").
 * One owner for the label rule so the CPU / Memory, GPU, and Process
 * selectors cannot drift apart.
 */
export const useArchivePeriodOptions = <P extends ArchivePeriod>(
  periods: readonly P[],
): ArchivePeriodOption<P>[] => {
  const { t } = useTranslation();

  return useMemo(
    () =>
      periods.map((period) => {
        const { count, unit } = describeArchivePeriod(period);
        return { label: `${count} ${t(`shared.time.${unit}`)}`, value: period };
      }),
    [periods, t],
  );
};
