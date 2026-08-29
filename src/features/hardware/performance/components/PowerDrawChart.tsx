import { LightningIcon } from "@phosphor-icons/react";
import { useAtomValue } from "jotai";
import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import type { CurveType } from "recharts/types/shape/Curve";
import {
  type ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import { chartConfig } from "@/features/hardware/consts/chart";
import {
  type PowerDrawHistory,
  powerDrawHistoryAtom,
} from "@/features/hardware/store/chart";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import type { LineGraphType, PowerDisplayTarget } from "@/rspc/bindings";

const curveTypes = {
  default: "monotone",
  step: "step",
  linear: "linear",
  basis: "basis",
} satisfies Record<LineGraphType, CurveType>;

const historyKey = {
  cpu: "cpuWatts",
  gpu: "gpuWatts",
  ane: "aneWatts",
  package: "packageWatts",
} as const satisfies Record<PowerDisplayTarget, keyof PowerDrawHistory>;

const fixedSeriesRgb = {
  ane: "168, 85, 247",
  package: "245, 158, 11",
} as const;

export const PowerDrawChart = ({
  showHeading = true,
  variant = "monitor",
}: {
  showHeading?: boolean;
  variant?: "monitor" | "panel";
} = {}) => {
  const { t } = useTranslation();
  const history = useAtomValue(powerDrawHistoryAtom);
  const { settings } = useSettingsAtom();
  const targets = settings.powerDisplayTargets;
  const colorRgb = useMemo(
    () => ({
      cpu: settings.lineGraphColor.cpu,
      gpu: settings.lineGraphColor.gpu,
      ...fixedSeriesRgb,
    }),
    [settings.lineGraphColor.cpu, settings.lineGraphColor.gpu],
  );
  const config = useMemo(
    () =>
      Object.fromEntries(
        targets.map((target) => [
          target,
          {
            label: t(`pages.performance.power.${target}`),
            color: `rgb(${colorRgb[target]})`,
          },
        ]),
      ) satisfies ChartConfig,
    [colorRgb, t, targets],
  );
  const data = useMemo(
    () =>
      Array.from({ length: chartConfig.historyLengthSec }, (_, index) => ({
        second: index - chartConfig.historyLengthSec + 1,
        cpu: history.cpuWatts.at(index - chartConfig.historyLengthSec) ?? null,
        gpu: history.gpuWatts.at(index - chartConfig.historyLengthSec) ?? null,
        ane: history.aneWatts.at(index - chartConfig.historyLengthSec) ?? null,
        package:
          history.packageWatts.at(index - chartConfig.historyLengthSec) ?? null,
      })),
    [history],
  );
  const maxValue = Math.max(
    0,
    ...targets.flatMap((target) =>
      history[historyKey[target]].filter(
        (value): value is number => value != null,
      ),
    ),
  );
  const yMax = Math.max(10, Math.ceil(maxValue / 10) * 10);

  return (
    <section
      className={cn(
        "flex min-h-30 flex-col",
        variant === "monitor" ? "flex-[2]" : "h-56 w-full px-4 pb-4",
      )}
      aria-label={t("pages.performance.panels.power")}
      data-testid="performance-power-graph"
    >
      <div
        className={cn(
          "mb-1 flex flex-wrap items-center gap-x-4 gap-y-1",
          showHeading && "px-1",
        )}
      >
        {showHeading ? (
          <div className="flex items-center gap-2 text-muted-foreground">
            <LightningIcon size={18} className="text-amber-400" />
            <h3 className="font-mono font-semibold text-[11px] uppercase tracking-[0.18em]">
              {t("pages.performance.panels.power")}
            </h3>
            <span className="text-[10px]">
              {t("pages.performance.shortWindow", {
                seconds: chartConfig.historyLengthSec,
              })}
            </span>
          </div>
        ) : null}
        <div
          className={cn(
            "flex flex-wrap gap-x-4 gap-y-1",
            showHeading && "ml-auto justify-end",
          )}
        >
          {targets.map((target) => {
            const value = history[historyKey[target]].at(-1) ?? null;
            return (
              <span
                key={target}
                className="flex items-center gap-1.5 font-mono text-[11px] text-muted-foreground tabular-nums"
              >
                <i
                  className="size-2 rounded-sm"
                  style={{ backgroundColor: `rgb(${colorRgb[target]})` }}
                />
                {t(`pages.performance.power.${target}`)}
                <strong className="text-foreground">
                  {value != null ? `${value.toFixed(1)} W` : "—"}
                </strong>
              </span>
            );
          })}
        </div>
      </div>
      <ChartContainer
        config={config}
        className={cn(
          "aspect-auto min-h-0 w-full flex-1",
          settings.lineGraphBorder &&
            "rounded-xl border-2 border-slate-400 py-6 dark:border-zinc-600",
        )}
      >
        <AreaChart
          data={data}
          margin={{ top: 4, right: 8, bottom: 0, left: 0 }}
        >
          <CartesianGrid
            horizontal={settings.lineGraphShowScale}
            vertical={false}
          />
          <XAxis
            dataKey="second"
            hide={!settings.lineGraphShowScale}
            tickFormatter={(value) =>
              value === 0
                ? t("pages.performance.now")
                : t("pages.performance.secondsAgo", { seconds: -value })
            }
            ticks={[-chartConfig.historyLengthSec + 1, -30, 0]}
          />
          <YAxis
            domain={[0, yMax]}
            hide={!settings.lineGraphShowScale}
            tickFormatter={(value) => `${value} W`}
            width={48}
          />
          {settings.lineGraphShowTooltip && (
            <ChartTooltip
              content={
                <ChartTooltipContent
                  formatter={(value, name) => (
                    <div className="flex min-w-32 items-center justify-between gap-4">
                      <span className="text-muted-foreground">
                        {config[name ?? ""]?.label ?? name}
                      </span>
                      <span className="font-medium font-mono tabular-nums">
                        {typeof value === "number"
                          ? `${value.toFixed(1)} W`
                          : "—"}
                      </span>
                    </div>
                  )}
                />
              }
            />
          )}
          {targets.map((target) => (
            <Area
              key={target}
              type={curveTypes[settings.lineGraphType]}
              dataKey={target}
              stroke={`rgb(${colorRgb[target]})`}
              strokeWidth={target === "package" ? 2.5 : 1.8}
              fill={
                settings.lineGraphFill
                  ? `rgba(${colorRgb[target]},${target === "package" ? 0.22 : 0.08})`
                  : "none"
              }
              connectNulls={false}
              isAnimationActive={false}
            />
          ))}
        </AreaChart>
      </ChartContainer>
    </section>
  );
};
