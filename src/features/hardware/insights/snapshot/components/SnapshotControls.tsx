import { CpuIcon, MemoryIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import { Slider } from "@/components/ui/slider";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { SnapshotPeriod, UsageRange } from "../types/snapshotType";
import { SelectMemoryMaxOption } from "./SnapshotForm";

interface SnapshotControlsProps {
  period: SnapshotPeriod;
  setPeriod: (period: SnapshotPeriod) => void;
  cpuRange: UsageRange;
  setCpuRange: (range: UsageRange) => void;
  memoryRange: UsageRange;
  setMemoryRange: (range: UsageRange) => void;
  selectedDataType: "cpu" | "memory";
  setSelectedDataType: (type: "cpu" | "memory") => void;
  selectedMemoryMaxMB: number;
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
}

const QuickPeriodButton = ({
  label,
  onClick,
  active,
}: {
  label: string;
  onClick: () => void;
  active?: boolean;
}) => (
  <Button
    variant={active ? "default" : "outline"}
    size="sm"
    onClick={onClick}
    className="whitespace-nowrap"
  >
    {label}
  </Button>
);

const RangeControl = ({
  icon,
  title,
  range,
  setRange,
  color,
  max = 100,
  formatValue,
}: {
  icon: React.ReactNode;
  title: string;
  range: UsageRange;
  setRange: (range: UsageRange) => void;
  color: string;
  max?: number;
  formatValue?: (value: number) => string;
}) => {
  const formatVal = formatValue || ((val) => `${val}%`);

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-2">
        <div style={{ color: `rgb(${color})` }}>{icon}</div>
        <Label className="font-medium">{title}</Label>
      </div>

      <div className="rounded-lg bg-muted/50 px-3 py-2">
        <div className="mb-2 flex justify-between text-muted-foreground text-sm">
          <span>{formatVal(range.value[0])}</span>
          <span>{formatVal(range.value[1])}</span>
        </div>

        <Slider
          value={range.value}
          onValueChange={(value) => {
            setRange({ ...range, value: value as [number, number] });
          }}
          min={0}
          max={max}
          step={1}
          className="w-full"
          aria-label={`${title} range selector`}
          aria-describedby={`${title.toLowerCase().replace(/\s+/g, "-")}-description`}
        />
        <div
          id={`${title.toLowerCase().replace(/\s+/g, "-")}-description`}
          className="sr-only"
        >
          Set the {title.toLowerCase()} range from {formatVal(range.value[0])}{" "}
          to {formatVal(range.value[1])}
        </div>

        <div className="mt-1 flex justify-between text-muted-foreground text-xs">
          <span>{formatVal(0)}</span>
          <span>{formatVal(Math.floor(max / 2))}</span>
          <span>{formatVal(max)}</span>
        </div>
      </div>
    </div>
  );
};

const DataTypeOption = ({
  value,
  icon,
  label,
  color,
}: {
  value: string;
  icon: React.ReactNode;
  label: string;
  color: string;
}) => (
  <Label className="flex cursor-pointer items-center gap-3 rounded-lg border border-transparent p-3 transition-colors hover:border-border hover:bg-muted/50">
    <RadioGroupItem
      value={value}
      className="h-4 w-4"
      aria-describedby={`${value}-description`}
    />
    <div className="flex items-center gap-2">
      <div style={{ color: `rgb(${color})` }} aria-hidden="true">
        {icon}
      </div>
      <span className="font-medium">{label}</span>
    </div>
    <span id={`${value}-description`} className="sr-only">
      View {label} usage data in the chart
    </span>
  </Label>
);

export const SnapshotControls = ({
  period,
  setPeriod,
  cpuRange,
  setCpuRange,
  memoryRange,
  setMemoryRange,
  selectedDataType,
  setSelectedDataType,
  selectedMemoryMaxMB,
  memoryMaxOption,
  setMemoryMaxOption,
  totalMemoryMB,
}: SnapshotControlsProps) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();

  const toLocalInputValue = (iso: string) => {
    const date = new Date(iso);
    const tzOffset = date.getTimezoneOffset() * 60000;
    const localDate = new Date(date.getTime() - tzOffset);
    return localDate.toISOString().slice(0, 16);
  };

  const setQuickPeriod = (duration: string) => {
    const now = new Date();
    let start: Date;

    switch (duration) {
      case "1h":
        start = new Date(now.getTime() - 60 * 60 * 1000);
        break;
      case "6h":
        start = new Date(now.getTime() - 6 * 60 * 60 * 1000);
        break;
      case "24h":
        start = new Date(now.getTime() - 24 * 60 * 60 * 1000);
        break;
      case "7d":
        start = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
        break;
      default:
        start = new Date(now.getTime() - 24 * 60 * 60 * 1000);
    }

    setPeriod({
      start: start.toISOString(),
      end: now.toISOString(),
    });
  };

  const formatMemoryValue = (mb: number) => {
    if (mb >= 1024) {
      return `${Math.round((mb / 1024) * 100) / 100}GB`;
    }
    return `${mb}MB`;
  };

  return (
    <div className="space-y-6">
      {/* Period Selection */}
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="flex-1">
          <Label className="mb-2 block font-medium text-sm">
            {t("shared.snapshot.timeRange")}
          </Label>
          <div className="flex items-center gap-2">
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
              className="flex-1"
            />
            <span className="text-muted-foreground">to</span>
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
              className="flex-1"
            />
          </div>
        </div>

        {/* Quick Period Buttons */}
        <div className="flex gap-2 overflow-x-auto pb-2 sm:flex-shrink-0 sm:pb-0">
          <QuickPeriodButton
            label={t("shared.snapshot.quickPeriods.1h")}
            onClick={() => setQuickPeriod("1h")}
          />
          <QuickPeriodButton
            label={t("shared.snapshot.quickPeriods.6h")}
            onClick={() => setQuickPeriod("6h")}
          />
          <QuickPeriodButton
            label={t("shared.snapshot.quickPeriods.24h")}
            onClick={() => setQuickPeriod("24h")}
          />
          <QuickPeriodButton
            label={t("shared.snapshot.quickPeriods.7d")}
            onClick={() => setQuickPeriod("7d")}
          />
        </div>
      </div>

      {/* Range Controls Section */}
      <div className="space-y-4">
        <div className="flex items-center justify-between border-t pt-4">
          <div className="">
            <div className="mb-3 flex items-center gap-2">
              <Label className="font-medium text-sm">
                {t("shared.snapshot.filtersSectionTitle")}
              </Label>
            </div>
            <p className="mb-4 text-muted-foreground text-sm">
              {t("shared.snapshot.filtersSectionDescription")}
            </p>
          </div>

          <div className="flex flex-col items-end">
            <SelectMemoryMaxOption
              memoryMaxOption={memoryMaxOption}
              setMemoryMaxOption={setMemoryMaxOption}
              totalMemoryMB={totalMemoryMB}
              className=""
            />
          </div>
        </div>

        <div className="grid grid-cols-1 gap-4 md:grid-cols-2 md:gap-6">
          <RangeControl
            icon={<CpuIcon size={20} />}
            title={`${t("shared.snapshot.cpuUsageFilter")} (${cpuRange.value[0]}% - ${cpuRange.value[1]}%)`}
            range={cpuRange}
            setRange={setCpuRange}
            color={settings.lineGraphColor.cpu}
            max={100}
          />
          <RangeControl
            icon={<MemoryIcon size={20} />}
            title={`${t("shared.snapshot.memoryUsageFilter")} (${formatMemoryValue(memoryRange.value[0])} - ${formatMemoryValue(memoryRange.value[1])})`}
            range={memoryRange}
            setRange={setMemoryRange}
            color={settings.lineGraphColor.memory}
            max={selectedMemoryMaxMB}
            formatValue={formatMemoryValue}
          />
        </div>
      </div>

      {/* Data Type Selection */}
      <div className="border-t pt-4">
        <Label className="mb-3 block font-medium text-sm">
          {t("shared.snapshot.dataType")}
        </Label>
        <RadioGroup
          className="flex flex-col gap-2 sm:flex-row sm:gap-6"
          value={selectedDataType}
          onValueChange={(value) =>
            setSelectedDataType(value as "cpu" | "memory")
          }
        >
          <DataTypeOption
            value="cpu"
            icon={<CpuIcon size={20} />}
            label="CPU"
            color={settings.lineGraphColor.cpu}
          />
          <DataTypeOption
            value="memory"
            icon={<MemoryIcon size={20} />}
            label="RAM"
            color={settings.lineGraphColor.memory}
          />
        </RadioGroup>
      </div>
    </div>
  );
};
