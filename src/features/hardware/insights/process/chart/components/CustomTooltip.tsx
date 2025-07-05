import { CpuIcon, MemoryIcon, TimerIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import type { TooltipProps } from "recharts";
import type {
  NameType,
  ValueType,
} from "recharts/types/component/DefaultTooltipContent";
import { bubbleChartColor } from "@/features/hardware/consts/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { formatBytes } from "@/lib/formatter";

export const CustomTooltip = ({
  active,
  payload,
}: TooltipProps<ValueType, NameType>) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();

  if (!active || !payload || payload.length === 0) return null;

  const data = payload[0].payload;

  return (
    <div className="grid min-w-[8rem] items-start gap-1.5 rounded-lg border border-neutral-200 bg-white px-2.5 py-1.5 text-xs shadow-xl dark:border-neutral-800 dark:bg-neutral-950">
      <div className="font-semibold">
        {data.name} (PID: {data.pid})
      </div>
      <div className="flex items-center gap-2">
        <TimerIcon size={16} color={bubbleChartColor} />
        <span className="text-neutral-500 dark:text-neutral-400">
          {t("shared.totalExecTime")}
        </span>
        <span className="font-medium font-mono text-neutral-950 tabular-nums dark:text-neutral-50">
          {data.x.toFixed(1)} {t("shared.time.minutes")}
        </span>
      </div>
      <div className="flex items-center gap-2">
        <CpuIcon size={16} color={`rgb(${settings.lineGraphColor.cpu})`} />
        <span className="text-neutral-500 dark:text-neutral-400">
          {t("shared.avgCpuUsage")}
        </span>
        <span className="font-medium font-mono text-neutral-950 tabular-nums dark:text-neutral-50">
          {data.y.toFixed(1)}%
        </span>
      </div>
      <div className="flex items-center gap-2">
        <MemoryIcon
          size={16}
          color={`rgb(${settings.lineGraphColor.memory})`}
        />
        <span className="text-neutral-500 dark:text-neutral-400">
          {t("shared.avgMemoryUsageValue")}
        </span>
        <span className="font-medium font-mono text-neutral-950 tabular-nums dark:text-neutral-50">
          {formatBytes(data.ram * 1024).join(" ")}
        </span>
      </div>
    </div>
  );
};
