import { useId } from "react";
import { twMerge } from "tailwind-merge";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
}: {
  range: UsageRange;
  setRange: (range: UsageRange) => void;
  label: string;
  className?: string;
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
        max={100}
        value={range.value}
        onValueChange={(value) => {
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
