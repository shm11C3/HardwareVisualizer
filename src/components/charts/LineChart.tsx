import { useSettingsAtom } from "@/atom/useSettingsAtom";
import type { sizeOptions } from "@/consts/chart";
import type { ChartDataType } from "@/types/hardwareDataType";
import { Cpu, GraphicsCard, Memory } from "@phosphor-icons/react";
import {
  Area,
  AreaChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { tv } from "tailwind-variants";
import CustomLegend, { type LegendItem } from "./CustomLegend";

type SingleChartProps = {
  labels: string[];
  chartData: number[];
  dataType: ChartDataType;
  size: (typeof sizeOptions)[number];
  lineGraphMix: false;
};

type MultiChartProps = {
  labels: string[];
  cpuData: number[];
  memoryData: number[];
  gpuData: number[];
  size: (typeof sizeOptions)[number];
  lineGraphMix: true;
};

const graphVariants = tv({
  base: "mt-5 mx-auto",
  variants: {
    size: {
      sm: "max-w-screen-sm",
      md: "max-w-screen-md",
      lg: "max-w-screen-lg",
      xl: "max-w-screen-xl",
      "2xl": "max-w-screen-2xl",
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

const graphAreaHeight: Record<(typeof sizeOptions)[number], number> = {
  sm: 200,
  md: 300,
  lg: 400,
  xl: 500,
  "2xl": 600,
};

const SingleLineChart = ({
  labels,
  chartData,
  dataType,
  size,
}: SingleChartProps) => {
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
      label: "Memory",
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
      <ResponsiveContainer
        className={chartAreaVariants({ border: settings.lineGraphBorder })}
        width="100%"
        height={graphAreaHeight[settings.graphSize]}
      >
        <AreaChart data={data}>
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
          <Tooltip
            isAnimationActive={false}
            contentStyle={{
              backgroundColor: "rgba(0, 0, 0, 0.7)",
              color: "#fff",
            }}
          />
          <Area
            type="monotone"
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
      </ResponsiveContainer>
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
  size,
}: MultiChartProps) => {
  const { settings } = useSettingsAtom();

  const data = labels.map((label, index) => ({
    name: label,
    cpu: cpuData[index],
    memory: memoryData[index],
    gpu: gpuData[index],
  }));

  const legendItems: LegendItem[] = [];

  if (settings.displayTargets.includes("cpu")) {
    legendItems.push({
      label: "CPU",
      icon: <Cpu size={20} color={`rgb(${settings.lineGraphColor.cpu})`} />,
    });
  }
  if (settings.displayTargets.includes("memory")) {
    legendItems.push({
      label: "Memory",
      icon: (
        <Memory size={20} color={`rgb(${settings.lineGraphColor.memory})`} />
      ),
    });
  }
  if (settings.displayTargets.includes("gpu")) {
    legendItems.push({
      label: "GPU",
      icon: (
        <GraphicsCard size={20} color={`rgb(${settings.lineGraphColor.gpu})`} />
      ),
    });
  }

  return (
    <div className={graphVariants({ size })}>
      <ResponsiveContainer
        className={chartAreaVariants({ border: settings.lineGraphBorder })}
        width="100%"
        height={graphAreaHeight[settings.graphSize]}
      >
        <AreaChart data={data} className="">
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
          <Tooltip
            isAnimationActive={false}
            contentStyle={{
              backgroundColor: "rgba(0, 0, 0, 0.7)",
              color: "#fff",
            }}
          />
          {settings.displayTargets.includes("cpu") && (
            <Area
              type="monotone"
              dataKey="cpu"
              stroke={`rgb(${settings.lineGraphColor.cpu})`}
              strokeWidth={2}
              fill={
                settings.lineGraphFill
                  ? `rgba(${settings.lineGraphColor.cpu},0.3)`
                  : "none"
              }
              isAnimationActive={false}
            />
          )}
          {settings.displayTargets.includes("memory") && (
            <Area
              type="monotone"
              dataKey="memory"
              stroke={`rgb(${settings.lineGraphColor.memory})`}
              strokeWidth={2}
              fill={
                settings.lineGraphFill
                  ? `rgba(${settings.lineGraphColor.memory},0.3)`
                  : "none"
              }
              isAnimationActive={false}
            />
          )}
          {settings.displayTargets.includes("gpu") && (
            <Area
              type="monotone"
              dataKey="gpu"
              stroke={`rgb(${settings.lineGraphColor.gpu})`}
              strokeWidth={2}
              fill={
                settings.lineGraphFill
                  ? `rgba(${settings.lineGraphColor.gpu},0.3)`
                  : "none"
              }
              isAnimationActive={false}
            />
          )}
        </AreaChart>
      </ResponsiveContainer>
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
 * @todo tooltip の表示を変更できるようにする
 */
export const LineChartComponent = (
  props: SingleChartProps | MultiChartProps,
) => {
  const { lineGraphMix } = props;

  return lineGraphMix ? (
    <MixLineChart {...props} />
  ) : (
    <SingleLineChart {...props} />
  );
};
