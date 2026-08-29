import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  type CoolingInsightPeriod,
  coolingInsightPeriods,
  isCoolingInsightPeriod,
} from "../types";

/**
 * The Cooling tab's single top-of-view period selector (#2018). Replaces the
 * nine separate per-chart selectors the legacy layout had, and is not the
 * `ArchivePeriod`-typed `SelectPeriod` those charts still use elsewhere -
 * 90d/1y have no archive-bucket equivalent.
 */
export const CoolingPeriodSelect = ({
  value,
  onChange,
}: {
  value: CoolingInsightPeriod;
  onChange: (value: CoolingInsightPeriod) => void;
}) => {
  const { t } = useTranslation();

  return (
    <Select
      value={value}
      onValueChange={(next) => {
        if (isCoolingInsightPeriod(next)) {
          onChange(next);
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
