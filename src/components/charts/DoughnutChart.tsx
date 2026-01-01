import {
  LightningIcon,
  MemoryIcon,
  SpeedometerIcon,
  ThermometerIcon,
} from "@phosphor-icons/react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import {
  Label,
  PolarGrid,
  PolarRadiusAxis,
  RadialBar,
  RadialBarChart,
} from "recharts";
import { type ChartConfig, ChartContainer } from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import { minOpacity } from "@/consts/style";
import type { HardwareDataType } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { Settings } from "@/features/settings/types/settingsType";
import { useWindowSize } from "@/hooks/useWindowSize";
import { cn } from "@/lib/utils";

type DoughnutChartProps =
  | {
      chartValue: number;
      usagePercentage: number;
      dataType: "memoryUsageValue";
      unit: string;
    }
  | {
      chartValue: number;
      dataType: Exclude<HardwareDataType, "memoryUsageValue">;
      unit?: never;
      usagePercentage?: never;
    };

const dataType2Units = (
  dataType: Exclude<HardwareDataType, "memoryUsageValue">,
  temperatureUnit: Settings["temperatureUnit"],
) => {
  const units = {
    usage: "%",
    temp: temperatureUnit === "C" ? "°C" : "°F",
    clock: "MHz",
  } as const;

  return units[dataType];
};

export const DoughnutChart = ({
  chartValue,
  dataType,
  unit,
  usagePercentage,
  className,
}: DoughnutChartProps & {
  className?: string;
}) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { isBreak } = useWindowSize();

  const isXl = isBreak("xl");

  const chartConfig: Record<
    HardwareDataType,
    { label: string; color: string }
  > = {
    usage: {
      label: t("shared.usage"),
      color: "hsl(var(--chart-2))",
    },
    temp: {
      label: t("shared.temperature.abbrev"),
      color: "hsl(var(--chart-3))",
    },
    clock: {
      label: t("shared.clock"),
      color: "hsl(var(--chart-4))",
    },
    memoryUsageValue: {
      label: t("shared.usageValue"),
      color: "hsl(var(--chart-5))",
    },
  } satisfies ChartConfig;

  const chartData = [
    { type: dataType, value: chartValue, fill: `var(--color-${dataType})` },
  ];

  const dataTypeIcons: Record<HardwareDataType, JSX.Element> = {
    usage: <LightningIcon className="mr-1" size={12} weight="duotone" />,
    temp: <ThermometerIcon className="mr-1" size={12} weight="duotone" />,
    clock: <SpeedometerIcon className="mr-1" size={12} weight="duotone" />,
    memoryUsageValue: (
      <MemoryIcon className="mr-1" size={12} weight="duotone" />
    ),
  };

  return (
    <ChartContainer
      config={chartConfig}
      className={cn("aspect-square max-h-[100px] xl:max-h-[200px]", className)}
    >
      {chartData[0].value != null ? (
        <RadialBarChart
          data={chartData}
          startAngle={0}
          endAngle={(() => {
            if (dataType === "memoryUsageValue") {
              return usagePercentage * 3.6;
            }

            return dataType === "temp" && settings.temperatureUnit === "F"
              ? ((chartValue - 32) / 1.8) * 3.6 // Convert Fahrenheit to Celsius and scale to 100 max
              : chartValue * 3.6;
          })()}
          innerRadius={isXl ? 50 : 35}
          outerRadius={isXl ? 60 : 45}
        >
          <PolarGrid
            gridType="circle"
            radialLines={false}
            stroke="none"
            className="first:fill-zinc-100 last:fill-[var(--chart-base)] dark:first:fill-muted"
            style={{
              opacity:
                settings.selectedBackgroundImg != null
                  ? Math.max(
                      (1 - settings.backgroundImgOpacity / 100) ** 2,
                      minOpacity,
                    )
                  : 1,
            }}
            polarRadius={isXl ? [70, 60] : [50, 42.5]}
          />
          <RadialBar dataKey="value" background cornerRadius={10} />
          <PolarRadiusAxis tick={false} tickLine={false} axisLine={false}>
            <Label
              content={({ viewBox }) => {
                if (viewBox && "cx" in viewBox && "cy" in viewBox) {
                  return (
                    <g>
                      {/* Display main value */}
                      <text
                        x={viewBox.cx}
                        y={viewBox.cy}
                        textAnchor="middle"
                        dominantBaseline="middle"
                        className="fill-foreground font-bold text-lg xl:text-2xl"
                      >
                        {`${chartValue}${dataType === "memoryUsageValue" ? unit : dataType2Units(dataType, settings.temperatureUnit)}`}
                      </text>
                      {/* Display label and icon */}
                      <foreignObject
                        x={(viewBox.cx || 0) - (isXl ? 42 : 38)}
                        y={(viewBox.cy || 0) + (isXl ? 25 : 15)}
                        width="80"
                        height="40"
                      >
                        <div className="flex items-center justify-center text-xs">
                          {dataTypeIcons[dataType]}
                          {isXl && <span>{chartConfig[dataType].label}</span>}
                        </div>
                      </foreignObject>
                    </g>
                  );
                }
              }}
            />
          </PolarRadiusAxis>
        </RadialBarChart>
      ) : (
        <div className="flex h-full items-center justify-center">
          <Skeleton className="h-32 w-32 rounded-full" />
        </div>
      )}
    </ChartContainer>
  );
};
