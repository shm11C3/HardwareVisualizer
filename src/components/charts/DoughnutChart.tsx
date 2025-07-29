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
}: DoughnutChartProps) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();

  const chartConfig: Record<
    HardwareDataType,
    { label: string; color: string }
  > = {
    usage: {
      label: t("shared.usage"),
      color: "hsl(var(--chart-2))",
    },
    temp: {
      label: t("shared.temperature"),
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
      className="aspect-square max-h-[200px]"
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
              ? ((chartValue - 32) / 1.8) * 3.6 // 華氏から摂氏に換算し、100を最大値としてスケール
              : chartValue * 3.6;
          })()}
          innerRadius={50}
          outerRadius={70}
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
            polarRadius={[70, 60]}
          />
          <RadialBar dataKey="value" background cornerRadius={10} />
          <PolarRadiusAxis tick={false} tickLine={false} axisLine={false}>
            <Label
              content={({ viewBox }) => {
                if (viewBox && "cx" in viewBox && "cy" in viewBox) {
                  return (
                    <g>
                      {/* メインの値表示 */}
                      <text
                        x={viewBox.cx}
                        y={viewBox.cy}
                        textAnchor="middle"
                        dominantBaseline="middle"
                        className="fill-foreground font-bold text-2xl"
                      >
                        {`${chartValue}${dataType === "memoryUsageValue" ? unit : dataType2Units(dataType, settings.temperatureUnit)}`}
                      </text>
                      {/* ラベルとアイコンの表示 */}
                      <foreignObject
                        x={(viewBox.cx || 0) - 42}
                        y={(viewBox.cy || 0) + 20}
                        width="80"
                        height="40"
                      >
                        <div className="flex items-center justify-center dark:text-muted-foreground ">
                          {dataTypeIcons[dataType]}
                          <span>{chartConfig[dataType].label}</span>
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
