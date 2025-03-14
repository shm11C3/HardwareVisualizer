import {
  type ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import type { SizeUnit } from "@/rspc/bindings";
import { useTranslation } from "react-i18next";
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from "recharts";

const chartKey = ["used", "free"] as const;

export type StorageBarChartData = {
  label: string;
  used: number;
  free: number;
};

export const StorageBarChart = ({
  chartData,
  unit,
}: {
  chartData: StorageBarChartData[];
  unit: SizeUnit;
}) => {
  const { t } = useTranslation();

  const StorageChartConfig: Record<
    (typeof chartKey)[number],
    { label: string; color: string }
  > = {
    used: {
      label: `${t("shared.used")} (${unit})`,
      color: "hsl(var(--chart-1))",
    },
    free: {
      label: `${t("shared.free")} (${unit})`,
      color: "hsl(var(--chart-2))",
    },
  } satisfies ChartConfig;

  return (
    <ChartContainer config={StorageChartConfig}>
      <BarChart layout="vertical" width={500} height={400} data={chartData}>
        <CartesianGrid horizontal={false} />
        <XAxis type="number" tickLine={false} axisLine={false} />
        <YAxis
          dataKey="label"
          type="category"
          tickLine={false}
          tickMargin={10}
          axisLine={false}
        />
        <ChartTooltip
          content={
            <ChartTooltipContent
              hideLabel
              showTotalValue
              totalLabel={`Total (${unit})`}
            />
          }
        />
        <ChartLegend content={<ChartLegendContent />} />
        {chartKey.map((key, i) => {
          const radius: [number, number, number, number] = (() => {
            // First item
            if (i === 0) {
              return [4, 0, 0, 4];
            }

            // Last item
            if (i === chartKey.length - 1) {
              return [0, 4, 4, 0];
            }

            // Middle item
            return [0, 0, 0, 0];
          })();

          return (
            <Bar
              key={key}
              dataKey={key}
              stackId="a"
              fill={`var(--color-${key})`}
              radius={radius}
            />
          );
        })}
      </BarChart>
    </ChartContainer>
  );
};
