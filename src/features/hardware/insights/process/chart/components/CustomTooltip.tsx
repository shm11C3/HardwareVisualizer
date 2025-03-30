import { formatBytes } from "@/lib/formatter";
import { useTranslation } from "react-i18next";
import type { TooltipProps } from "recharts";
import type {
  NameType,
  ValueType,
} from "recharts/types/component/DefaultTooltipContent";

export const CustomTooltip = ({
  active,
  payload,
}: TooltipProps<ValueType, NameType>) => {
  const { t } = useTranslation();

  if (!active || !payload || payload.length === 0) return null;

  const data = payload[0].payload;

  return (
    <div className="grid min-w-[8rem] items-start gap-1.5 rounded-lg border border-neutral-200 bg-white px-2.5 py-1.5 text-xs shadow-xl dark:border-neutral-800 dark:bg-neutral-950">
      <div className="font-semibold">
        {data.name} (PID: {data.pid})
      </div>
      <div className="flex items-center gap-2">
        <span className="text-neutral-500 dark:text-neutral-400">
          {t("shared.execTime")}
        </span>
        <span className="font-mono font-medium tabular-nums text-neutral-950 dark:text-neutral-50">
          {data.x.toFixed(1)} {t("shared.time.minutes")}
        </span>
      </div>
      <div className="flex items-center gap-2">
        <span className="text-neutral-500 dark:text-neutral-400">
          {t("shared.cpuUsage")}
        </span>
        <span className="font-mono font-medium tabular-nums text-neutral-950 dark:text-neutral-50">
          {data.y.toFixed(1)}%
        </span>
      </div>
      <div className="flex items-center gap-2">
        <span className="text-neutral-500 dark:text-neutral-400">
          {t("shared.memoryUsageValue")}
        </span>
        <span className="font-mono font-medium tabular-nums text-neutral-950 dark:text-neutral-50">
          {formatBytes(data.ram * 1024).join(" ")}
        </span>
      </div>
    </div>
  );
};
