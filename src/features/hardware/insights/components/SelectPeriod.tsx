import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  type ArchivePeriod,
  normalizeArchivePeriod,
} from "../utils/archivePeriod";

export const SelectPeriod = ({
  options,
  selected,
  onChange,
  showDefaultOption,
}: {
  options: { label: string; value: ArchivePeriod }[];
  selected: ArchivePeriod | null;
  onChange: (value: ArchivePeriod) => void;
  showDefaultOption?: boolean;
}) => {
  const { t } = useTranslation();

  return (
    <Select
      value={String(selected ?? "not-selected")}
      onValueChange={(value) => {
        const period = normalizeArchivePeriod(value);
        if (period != null) {
          onChange(period);
        }
      }}
    >
      <SelectTrigger className="w-[180px]">
        <SelectValue placeholder="Select Period" />
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
