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
  return (
    <div className={twMerge("flex items-center gap-2", className)}>
      <Input
        type="datetime-local"
        value={period.start.slice(0, 16)}
        onChange={(e) =>
          setPeriod({
            ...period,
            start: e.target.value,
          })
        }
      />
      ~
      <Input
        type="datetime-local"
        value={period.end.slice(0, 16)}
        onChange={(e) =>
          setPeriod({
            ...period,
            end: e.target.value,
          })
        }
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
