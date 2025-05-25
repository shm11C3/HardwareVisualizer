import {
  type ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import type { sizeOptions } from "@/features/hardware/consts/chart";
import {
  type ChartDataType,
  type GpuDataType,
  isChartDataType,
} from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { cn } from "@/lib/utils";
import type { LineGraphType } from "@/rspc/bindings";
import { CpuIcon, GraphicsCardIcon, MemoryIcon } from "@phosphor-icons/react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import type { CurveType } from "recharts/types/shape/Curve";
import { tv } from "tailwind-variants";
import CustomLegend, { type LegendItem } from "./CustomLegend";

type ChartProps = {
  labels: string[];
  size: (typeof sizeOptions)[number];
};

type SingleChartProps = {
  chartData: (number | null)[];
  dataType: ChartDataType | GpuDataType;
  lineGraphMix: false;
} & ChartProps;

type MultiChartProps = {
  cpuData: number[];
  memoryData: number[];
  gpuData: number[];
  lineGraphMix: true;
} & ChartProps;

const lineGraphType2RechartsCurveType: Record<LineGraphType, CurveType> = {
  default: "monotone",
  step: "step",
  linear: "linear",
  basis: "basis",
};

const graphVariants = tv({
  base: "mx-auto",
  variants: {
    size: {
      sm: "max-w-(--breakpoint-sm)",
      md: "max-w-(--breakpoint-md)",
      lg: "max-w-(--breakpoint-lg)",
      xl: "max-w-(--breakpoint-xl)",
      "2xl": "max-w-(--breakpoint-2xl)",
    },
  },
  defaultVariants: {
    size: "xl",
  },
});

const chartAreaVariants = tv({
  variants: {
    border: {
      true: "border-2 rounded-xl border-slate-400 dark:border-zinc-600 py-6",
    },
  },
});

export const SingleLineChart = ({
  labels,
  chartData,
  dataType,
  chartConfig,
  size,
  border,
  lineGraphShowScale,
  lineGraphShowTooltip,
  lineGraphType,
  lineGraphShowLegend,
  dataKey,
  range = [0, 100],
  width,
  height,
  className,
}: SingleChartProps & { chartConfig: ChartConfig } & {
  border: boolean;
  lineGraphShowScale: boolean;
  lineGraphShowTooltip: boolean;
  lineGraphType: LineGraphType;
  lineGraphShowLegend: boolean;
  dataKey: string;
  range?: [number, number];
  width?: number | string;
  height?: number | string;
  className?: string;
}) => {
  const { settings } = useSettingsAtom();

  const data = labels.map((label, index) => ({
    name: label,
    [dataKey]: chartData[index],
  }));

  const legendItems: Record<ChartDataType, LegendItem> = {
    cpu: {
      label: "CPU",
      icon: <CpuIcon size={20} color={`rgb(${settings.lineGraphColor.cpu})`} />,
    },
    memory: {
      label: "RAM",
      icon: (
        <MemoryIcon
          size={20}
          color={`rgb(${settings.lineGraphColor.memory})`}
        />
      ),
    },
    gpu: {
      label: "GPU",
      icon: (
        <GraphicsCardIcon
          size={20}
          color={`rgb(${settings.lineGraphColor.gpu})`}
        />
      ),
    },
  };

  // [TODO] 選択した範囲を横に移動できるようにする
  return (
    <div className={cn(graphVariants({ size }), className)}>
      <ChartContainer
        className={chartAreaVariants({ border })}
        config={chartConfig}
        style={{
          ...(width && { width }),
          ...(height && { height }),
        }}
      >
        <AreaChart data={data}>
          <CartesianGrid horizontal={lineGraphShowScale} vertical={false} />
          <XAxis dataKey="name" hide={!lineGraphShowScale} />
          <YAxis
            domain={range}
            hide={!lineGraphShowScale}
            tick={{
              fill: { light: "#77777", dark: "#fff" }[settings.theme],
            }}
            stroke={
              { light: "#77777", dark: "rgba(255, 255, 255, 0.2)" }[
                settings.theme
              ]
            }
            tickCount={12}
          />
          {lineGraphShowTooltip && (
            <ChartTooltip content={<ChartTooltipContent />} />
          )}
          <Area
            type={lineGraphType2RechartsCurveType[lineGraphType]}
            dataKey={dataKey}
            stroke={`rgb(${chartConfig[dataType].color})`}
            strokeWidth={2}
            fill={
              settings.lineGraphFill
                ? `rgba(${chartConfig[dataType].color},0.3)`
                : "none"
            }
            isAnimationActive={false}
          />
        </AreaChart>
      </ChartContainer>
      {lineGraphShowLegend && isChartDataType(dataType) && (
        <div className="mt-4 mb-2 flex justify-center">
          <CustomLegend item={legendItems[dataType]} />
        </div>
      )}
    </div>
  );
};

const MixLineChart = ({
  labels,
  cpuData,
  memoryData,
  gpuData,
  chartConfig,
  size,
}: MultiChartProps & { chartConfig: ChartConfig }) => {
  const { settings } = useSettingsAtom();

  const displayOrder = ["cpu", "memory", "gpu"];

  const sortedDisplayTargets = settings.displayTargets
    .slice()
    .sort((a, b) => displayOrder.indexOf(a) - displayOrder.indexOf(b));

  const data = labels.map((label, index) => ({
    name: label,
    cpu: cpuData[index],
    memory: memoryData[index],
    gpu: gpuData[index],
  }));

  const iconMap: Record<string, JSX.Element> = {
    cpu: <CpuIcon size={20} color={`rgb(${settings.lineGraphColor.cpu})`} />,
    memory: (
      <MemoryIcon size={20} color={`rgb(${settings.lineGraphColor.memory})`} />
    ),
    gpu: (
      <GraphicsCardIcon
        size={20}
        color={`rgb(${settings.lineGraphColor.gpu})`}
      />
    ),
  };

  const legendItems: LegendItem[] = sortedDisplayTargets.map((target) => ({
    label: { cpu: "CPU", memory: "RAM", gpu: "GPU" }[target],
    icon: iconMap[target],
  }));

  return (
    <div className={graphVariants({ size })}>
      <ChartContainer
        className={chartAreaVariants({ border: settings.lineGraphBorder })}
        config={chartConfig}
      >
        <AreaChart data={data}>
          <CartesianGrid
            horizontal={settings.lineGraphShowScale}
            vertical={false}
          />
          <XAxis dataKey="name" hide={!settings.lineGraphShowScale} />
          <YAxis
            domain={[0, 100]}
            hide={!settings.lineGraphShowScale}
            tick={{
              fill: { light: "#77777", dark: "#fff" }[settings.theme],
            }}
            stroke={
              { light: "#77777", dark: "rgba(255, 255, 255, 0.2)" }[
                settings.theme
              ]
            }
            tickCount={12}
          />
          {settings.lineGraphShowTooltip && (
            <ChartTooltip content={<ChartTooltipContent />} />
          )}
          {sortedDisplayTargets.map((areaData) => (
            <Area
              key={areaData}
              type={lineGraphType2RechartsCurveType[settings.lineGraphType]}
              dataKey={areaData}
              stroke={`rgb(${settings.lineGraphColor[areaData]})`}
              strokeWidth={2}
              fill={
                settings.lineGraphFill
                  ? `rgba(${settings.lineGraphColor[areaData]},0.3)`
                  : "none"
              }
              isAnimationActive={false}
            />
          ))}
        </AreaChart>
      </ChartContainer>
      {settings.lineGraphShowLegend && (
        <div className="mt-4 mb-2 flex justify-center">
          {legendItems.map((item) => (
            <CustomLegend key={item.label} item={item} />
          ))}
        </div>
      )}
    </div>
  );
};

/**
 * @todo `type="monotone"` を変更できるようにする
 * @todo tooltip の表示/非表示を変更できるようにする
 */
export const LineChartComponent = (
  props: SingleChartProps | MultiChartProps,
) => {
  const { t } = useTranslation();
  const { settings } = useSettingsAtom();
  const { lineGraphMix } = props;

  const chartConfig: Record<ChartDataType, { label: string; color: string }> = {
    cpu: {
      label: "CPU",
      color: settings.lineGraphColor.cpu,
    },
    memory: {
      label: "RAM",
      color: settings.lineGraphColor.memory,
    },
    gpu: {
      label: "GPU",
      color: settings.lineGraphColor.gpu,
    },
  } satisfies ChartConfig;

  return lineGraphMix ? (
    <MixLineChart {...props} chartConfig={chartConfig} />
  ) : (
    <SingleLineChart
      className="mt-5"
      {...props}
      chartConfig={chartConfig}
      border={settings.lineGraphBorder}
      lineGraphShowScale={settings.lineGraphShowScale}
      lineGraphShowTooltip={settings.lineGraphShowTooltip}
      lineGraphType={settings.lineGraphType}
      lineGraphShowLegend={settings.lineGraphShowLegend}
      dataKey={`${t("shared.usage")} (%)`}
    />
  );
};
