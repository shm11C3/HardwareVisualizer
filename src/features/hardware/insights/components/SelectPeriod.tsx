import { useTranslation } from "react-i18next";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { ArchivePeriod } from "@/features/hardware/insights/utils/archivePeriod";

/**
 * A period picker over exactly the options it is given. The chosen value is
 * resolved back to one of those options, so a caller offering a subset (Process
 * Insight stops at 30 days) can never receive a period it did not offer.
 */
export const SelectPeriod = <P extends ArchivePeriod>({
  options,
  selected,
  onChange,
  showDefaultOption,
}: {
  options: { label: string; value: P }[];
  selected: P | null;
  onChange: (value: P) => void;
  showDefaultOption?: boolean;
}) => {
  const { t } = useTranslation();

  return (
    <Select
      value={String(selected ?? "not-selected")}
      onValueChange={(value) => {
        const option = options.find(
          (candidate) => String(candidate.value) === value,
        );
        if (option != null) {
          onChange(option.value);
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
