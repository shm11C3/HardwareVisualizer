import {
  LightningIcon,
  MemoryIcon,
  SpeedometerIcon,
  ThermometerIcon,
} from "@phosphor-icons/react";
import type { JSX } from "react";
import { useTranslation } from "react-i18next";
import {
  gaugeFraction,
  gaugeRingDash,
} from "@/components/charts/gaugeGeometry";
import { Skeleton } from "@/components/ui/skeleton";
import { minOpacity } from "@/consts/style";
import type { HardwareDataType } from "@/features/hardware/types/hardwareDataType";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { Settings } from "@/features/settings/types/settingsType";
import { useWindowSize } from "@/hooks/useWindowSize";
import { cn } from "@/lib/utils";

type DoughnutChartProps =
  | {
      chartValue: number | null;
      usagePercentage: number;
      dataType: "memoryUsageValue";
      unit: string;
    }
  | {
      chartValue: number | null;
      dataType: Exclude<HardwareDataType, "memoryUsageValue">;
      unit?: never;
      usagePercentage?: never;
    };

/**
 * Gauge tween length. Must stay below the 1Hz hardware update interval, and
 * shorter still keeps the per-tick repaint cost down.
 */
export const gaugeAnimationDurationMs = 300;

/**
 * Ring and backing-disc geometry per breakpoint.
 *
 * The view box matches the container's pixel size at each breakpoint rather
 * than being fixed, because these radii are the ones the Recharts gauge used
 * and Recharts sized its canvas to the container. Drawing the compact radii
 * into the larger box would scale the whole gauge to half the surface.
 */
const ringLayout = {
  xl: { viewBox: 200, radius: 55, width: 10, outerDisc: 70, innerDisc: 60 },
  base: { viewBox: 100, radius: 40, width: 10, outerDisc: 50, innerDisc: 42.5 },
} as const;

const dataTypeColors: Record<HardwareDataType, string> = {
  usage: "hsl(var(--chart-2))",
  temp: "hsl(var(--chart-3))",
  clock: "hsl(var(--chart-4))",
  memoryUsageValue: "hsl(var(--chart-5))",
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
  const layout = isXl ? ringLayout.xl : ringLayout.base;
  const center = layout.viewBox / 2;

  const labels: Record<HardwareDataType, string> = {
    usage: t("shared.usage"),
    temp: t("shared.temperature.abbrev"),
    clock: t("shared.clock"),
    memoryUsageValue: t("shared.usageValue"),
  };

  const dataTypeIcons: Record<HardwareDataType, JSX.Element> = {
    usage: <LightningIcon className="mr-1" size={12} weight="duotone" />,
    temp: <ThermometerIcon className="mr-1" size={12} weight="duotone" />,
    clock: <SpeedometerIcon className="mr-1" size={12} weight="duotone" />,
    memoryUsageValue: (
      <MemoryIcon className="mr-1" size={12} weight="duotone" />
    ),
  };

  const discOpacity =
    settings.selectedBackgroundImg != null
      ? Math.max((1 - settings.backgroundImgOpacity / 100) ** 2, minOpacity)
      : 1;

  const containerClassName = cn(
    "aspect-square max-h-[100px] xl:max-h-[200px]",
    className,
  );

  if (chartValue == null) {
    return (
      <div
        className={cn(containerClassName, "flex items-center justify-center")}
      >
        {/* Sized per breakpoint: the compact surface is 100px square, which a
            fixed 128px placeholder would overflow. */}
        <Skeleton className="h-24 w-24 rounded-full xl:h-32 xl:w-32" />
      </div>
    );
  }

  const label = labels[dataType];
  const dash = gaugeRingDash(
    gaugeFraction({
      chartValue,
      dataType,
      usagePercentage,
      temperatureUnit: settings.temperatureUnit,
    }),
    layout.radius,
  );

  return (
    <div className={containerClassName}>
      <svg
        className="h-full w-full"
        viewBox={`0 0 ${layout.viewBox} ${layout.viewBox}`}
        role="presentation"
      >
        <circle
          cx={center}
          cy={center}
          r={layout.outerDisc}
          className="fill-zinc-100 dark:fill-muted"
          style={{ opacity: discOpacity }}
        />
        <circle
          cx={center}
          cy={center}
          r={layout.innerDisc}
          className="fill-[var(--chart-base)]"
          style={{ opacity: discOpacity }}
        />
        {/*
          The ring starts at 3 o'clock and fills anticlockwise. A stroked
          circle runs clockwise, so the group is mirrored on Y rather than the
          arc being rebuilt as a path.
        */}
        <g
          transform={`translate(${center}, ${center}) scale(1, -1) translate(${-center}, ${-center})`}
        >
          {/*
            Hardware metrics arrive every second, and the gauge used to tween
            with a JS animation that re-rendered the chart on every frame.
            Transitioning the dash offset hands those frames to the compositor:
            React renders once per tick, and the tween still has to finish
            inside the interval or it would restart before settling.
          */}
          <circle
            cx={center}
            cy={center}
            r={layout.radius}
            fill="none"
            stroke={dataTypeColors[dataType]}
            strokeWidth={layout.width}
            strokeLinecap="round"
            strokeDasharray={dash.strokeDasharray}
            strokeDashoffset={dash.strokeDashoffset}
            className="transition-[stroke-dashoffset] ease-out motion-reduce:transition-none"
            style={{ transitionDuration: `${gaugeAnimationDurationMs}ms` }}
          />
        </g>

        {/*
          The readout lives inside the view box so it scales with the gauge.
          The container is half size below xl, and CSS pixel sizes on an HTML
          overlay would not follow it — the text would swamp the ring.
        */}
        <text
          x={center}
          y={center}
          textAnchor="middle"
          dominantBaseline="middle"
          className="fill-foreground font-bold text-lg xl:text-2xl"
        >
          {`${chartValue}${dataType === "memoryUsageValue" ? unit : dataType2Units(dataType, settings.temperatureUnit)}`}
        </text>
        <foreignObject
          x={center - (isXl ? 42 : 38)}
          y={center + (isXl ? 25 : 15)}
          width="80"
          height="40"
        >
          <div className="flex items-center justify-center text-xs">
            {dataTypeIcons[dataType]}
            {isXl && label.length <= 5 && <span>{label}</span>}
          </div>
        </foreignObject>
      </svg>
    </div>
  );
};
