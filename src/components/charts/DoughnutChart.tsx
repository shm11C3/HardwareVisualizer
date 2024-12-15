import { useSettingsAtom } from "@/atom/useSettingsAtom";
import { type ChartConfig, ChartContainer } from "@/components/ui/chart";
import { Skeleton } from "@/components/ui/skeleton";
import type { HardwareDataType } from "@/types/hardwareDataType";
import { Lightning, Speedometer, Thermometer } from "@phosphor-icons/react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import {
  Label,
  PolarGrid,
  PolarRadiusAxis,
  RadialBar,
  RadialBarChart,
} from "recharts";

export const DoughnutChart = ({
  chartValue,
  dataType,
}: {
  chartValue: number;
  dataType: HardwareDataType;
}) => {
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
  } satisfies ChartConfig;

  const chartData = [
    { type: dataType, value: chartValue, fill: `var(--color-${dataType})` },
  ];

  const dataTypeIcons: Record<HardwareDataType, JSX.Element> = {
    usage: <Lightning className="mr-1" size={12} weight="duotone" />,
    temp: <Thermometer className="mr-1" size={12} weight="duotone" />,
    clock: <Speedometer className="mr-1" size={12} weight="duotone" />,
  };

  const dataTypeUnits: Record<HardwareDataType, string> = {
    usage: "%",
    temp: "°C",
    clock: "MHz",
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
          endAngle={chartValue * 3.6}
          innerRadius={50}
          outerRadius={70}
        >
          <PolarGrid
            gridType="circle"
            radialLines={false}
            stroke="none"
            className="first:fill-zinc-300 dark:first:fill-muted  last:fill-zinc-200 last:dark:fill-gray-900"
            style={{
              opacity: (1 - settings.backgroundImgOpacity / 100) ** 2,
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
                        className="fill-foreground text-2xl font-bold"
                      >
                        {`${chartValue}${dataTypeUnits[dataType]}`}
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
        <div className="flex items-center justify-center h-full">
          <Skeleton className="w-32 h-32 rounded-full" />
        </div>
      )}
    </ChartContainer>
  );
};
