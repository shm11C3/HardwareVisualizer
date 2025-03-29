import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useTranslation } from "react-i18next";
import type { archivePeriods } from "../../consts/chart";

export const SelectPeriod = ({
  options,
  selected,
  onChange,
  showDefaultOption,
}: {
  options: { label: string; value: keyof typeof archivePeriods }[];
  selected: keyof typeof archivePeriods | null;
  onChange: (value: (typeof archivePeriods)[number]) => void;
  showDefaultOption?: boolean;
}) => {
  const { t } = useTranslation();

  return (
    <Select
      value={String(selected ?? "not-selected")}
      onValueChange={(value) =>
        onChange(value as unknown as (typeof archivePeriods)[number])
      }
    >
      <SelectTrigger className="w-[180px]">
        <SelectValue placeholder="Temperature Unit" />
      </SelectTrigger>
      <SelectContent>
        {showDefaultOption && (
          <SelectItem key="" value="not-selected">
            {t("shared.select")}
          </SelectItem>
        )}
        {options.map((option) => (
          <SelectItem key={String(option.value)} value={String(option.value)}>
            {option.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
};
