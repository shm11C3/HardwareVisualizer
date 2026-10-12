import { useAtomValue } from "jotai";
import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSetCoolingInsightPeriod } from "@/features/hardware/insights/cooling/hooks/useSetCoolingInsightPeriod";
import {
  coolingInsightPeriods,
  isCoolingInsightPeriod,
} from "@/features/hardware/insights/cooling/types";
import { coolingInsightPeriodAtom } from "@/features/hardware/store/coolingPeriod";

/**
 * The Cooling tab's single top-of-view period selector (#2018). Replaces the
 * nine separate per-chart selectors the legacy layout had, and is not the
 * `ArchivePeriod`-typed `SelectPeriod` those charts still use elsewhere -
 * 90d/1y have no archive-bucket equivalent.
 */
export const CoolingPeriodSelect = () => {
  const { t } = useTranslation();
  const value = useAtomValue(coolingInsightPeriodAtom);
  const setPeriod = useSetCoolingInsightPeriod();

  return (
    <Select
      value={value}
      onValueChange={(next) => {
        if (isCoolingInsightPeriod(next)) {
          setPeriod(next);
        }
      }}
    >
      <SelectTrigger className="w-[180px]" data-testid="cooling-period-select">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {coolingInsightPeriods.map((period) => (
          <SelectItem key={period} value={period}>
            {t(`pages.insights.cooling.periods.${period}`)}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
};
