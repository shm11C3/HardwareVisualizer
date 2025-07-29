import { useId } from "react";
import { useTranslation } from "react-i18next";
import { twMerge } from "tailwind-merge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Slider } from "@/components/ui/slider";
import type { SnapshotPeriod, UsageRange } from "../types/snapshotType";

export const SelectPeriod = ({
  period,
  setPeriod,
  className,
}: {
  period: SnapshotPeriod;
  setPeriod: (period: SnapshotPeriod) => void;
  className?: string;
}) => {
  const toLocalInputValue = (iso: string) => {
    // ISOストリング（UTC）をローカル時間のinput値に変換
    const date = new Date(iso);
    // toISOString()はUTC時間を返すので、getTimezoneOffset()を使ってローカル時間に調整
    const tzOffset = date.getTimezoneOffset() * 60000;
    const localDate = new Date(date.getTime() - tzOffset);
    return localDate.toISOString().slice(0, 16);
  };

  return (
    <div className={twMerge("flex items-center gap-2", className)}>
      <Input
        type="datetime-local"
        value={toLocalInputValue(period.start)}
        onChange={(e) => {
          const localDate = new Date(e.target.value);

          setPeriod({
            ...period,
            start: localDate.toISOString(),
          });
        }}
      />
      ~
      <Input
        type="datetime-local"
        value={toLocalInputValue(period.end)}
        onChange={(e) => {
          const localDate = new Date(e.target.value);

          setPeriod({
            ...period,
            end: localDate.toISOString(),
          });
        }}
      />
    </div>
  );
};

export const SelectRange = ({
  range,
  setRange,
  label,
  className,
  max = 100,
}: {
  range: UsageRange;
  setRange: (range: UsageRange) => void;
  label: string;
  className?: string;
  max?: number;
}) => {
  const formId = useId();
  return (
    <div className={twMerge("flex flex-col", className)}>
      <Label htmlFor={formId} className="mb-3">
        {label}
      </Label>
      <Slider
        id={formId}
        aria-label={label}
        min={0}
        max={max}
        value={range.value}
        onValueChange={(value: [number, number]) => {
          setRange({
            ...range,
            value: [value[0], value[1]],
          });
        }}
        className="w-full"
      />
    </div>
  );
};

export const SelectMemoryMaxOption = ({
  memoryMaxOption,
  setMemoryMaxOption,
  totalMemoryMB,
  className,
}: {
  memoryMaxOption:
    | "128MB"
    | "256MB"
    | "512MB"
    | "1GB"
    | "2GB"
    | "8GB"
    | "device";

  setMemoryMaxOption: (
    option: "128MB" | "256MB" | "512MB" | "1GB" | "2GB" | "8GB" | "device",
  ) => void;
  totalMemoryMB: number;
  className?: string;
}) => {
  const formId = useId();
  const { t } = useTranslation();

  const formatMemorySize = (mb: number) => {
    if (mb >= 1024) {
      return `${Math.round(mb / 1024)}GB`;
    }
    return `${mb}MB`;
  };

  return (
    <div className={twMerge("flex items-center gap-3", className)}>
      <Label htmlFor={formId}>{t("shared.memoryRangeMax")}</Label>
      <Select
        value={memoryMaxOption}
        onValueChange={(value) =>
          setMemoryMaxOption(
            value as
              | "128MB"
              | "256MB"
              | "512MB"
              | "1GB"
              | "2GB"
              | "8GB"
              | "device",
          )
        }
      >
        <SelectTrigger id={formId}>
          <SelectValue placeholder={t("shared.selectMemoryRangeMax")} />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="128MB">128MB</SelectItem>
          <SelectItem value="256MB">256MB</SelectItem>
          <SelectItem value="512MB">512MB</SelectItem>
          <SelectItem value="1GB">1GB</SelectItem>
          <SelectItem value="2GB">2GB</SelectItem>
          <SelectItem value="8GB">8GB</SelectItem>
          <SelectItem value="device">
            {t("shared.deviceCapacity")} ({formatMemorySize(totalMemoryMB)})
          </SelectItem>
        </SelectContent>
      </Select>
    </div>
  );
};
