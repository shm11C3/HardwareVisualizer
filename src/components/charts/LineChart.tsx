import { useSettingsAtom } from "@/atom/useSettingsAtom";
import {
  type ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import type { sizeOptions } from "@/consts/chart";
import type { LineGraphType } from "@/rspc/bindings";
import type { ChartDataType } from "@/types/hardwareDataType";
import { Cpu, GraphicsCard, Memory } from "@phosphor-icons/react";
import type { JSX } from "react";
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from "recharts";
import type { CurveType } from "recharts/types/shape/Curve";
import { tv } from "tailwind-variants";
import CustomLegend, { type LegendItem } from "./CustomLegend";

type ChartProps = {
  labels: string[];
  size: (typeof sizeOptions)[number];
};

type SingleChartProps = {
  chartData: number[];
  dataType: ChartDataType;
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
  base: "mt-5 mx-auto",
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

const SingleLineChart = ({
  labels,
  chartData,
  dataType,
  chartConfig,
  size,
}: SingleChartProps & { chartConfig: ChartConfig }) => {
  const { settings } = useSettingsAtom();

  const data = labels.map((label, index) => ({
    name: label,
    usage: chartData[index],
  }));

  const legendItems: Record<ChartDataType, LegendItem> = {
    cpu: {
      label: "CPU",
      icon: <Cpu size={20} color={`rgb(${settings.lineGraphColor.cpu})`} />,
    },
    memory: {
      label: "RAM",
      icon: (
        <Memory size={20} color={`rgb(${settings.lineGraphColor.memory})`} />
      ),
    },
    gpu: {
      label: "GPU",
      icon: (
        <GraphicsCard size={20} color={`rgb(${settings.lineGraphColor.gpu})`} />
      ),
    },
  };

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
          <Area
            type={lineGraphType2RechartsCurveType[settings.lineGraphType]}
            dataKey="usage"
            stroke={`rgb(${settings.lineGraphColor[dataType]})`}
            strokeWidth={2}
            fill={
              settings.lineGraphFill
                ? `rgba(${settings.lineGraphColor[dataType]},0.3)`
                : "none"
            }
            isAnimationActive={false}
          />
        </AreaChart>
      </ChartContainer>
      {settings.lineGraphShowLegend && (
        <div className="flex justify-center mt-4 mb-2">
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
    cpu: <Cpu size={20} color={`rgb(${settings.lineGraphColor.cpu})`} />,
    memory: (
      <Memory size={20} color={`rgb(${settings.lineGraphColor.memory})`} />
    ),
    gpu: (
      <GraphicsCard size={20} color={`rgb(${settings.lineGraphColor.gpu})`} />
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
        <div className="flex justify-center mt-4 mb-2">
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
  const { settings } = useSettingsAtom();
  const { lineGraphMix } = props;

  const chartConfig: Record<ChartDataType, { label: string; color: string }> = {
    cpu: {
      label: "CPU",
      color: `rgb(${settings.lineGraphColor.cpu})`,
    },
    memory: {
      label: "RAM",
      color: `rgb(${settings.lineGraphColor.memory})`,
    },
    gpu: {
      label: "GPU",
      color: `rgb(${settings.lineGraphColor.gpu})`,
    },
  } satisfies ChartConfig;

  return lineGraphMix ? (
    <MixLineChart {...props} chartConfig={chartConfig} />
  ) : (
    <SingleLineChart {...props} chartConfig={chartConfig} />
  );
};
