import { useState } from "react";
import {
  buildSparklinePath,
  sparklineGridLines,
  sparklineViewBox,
} from "@/components/charts/sparklinePath";
import { cn } from "@/lib/utils";
import type { LineGraphType } from "@/rspc/bindings";

const gridTickCount = 5;

type SparklineProps = {
  values: (number | null)[];
  /** `R, G, B` triplet, as stored in the line graph color settings. */
  colorRgb: string;
  lineGraphType: LineGraphType;
  fill: boolean;
  showScale: boolean;
  tooltip?: { label: string; format: (value: number) => string };
  range?: [number, number];
  className?: string;
};

/**
 * A live line chart for the always-on 1 Hz surfaces.
 *
 * These charts update every second and are rendered many at a time (one per
 * logical core on the CPU screen), where a full Recharts component tree per
 * tick dominates the visible-window render cost (#1581). Here a tick only
 * mutates the `d` of two `<path>`s, so React reconciles two attributes and
 * the engine re-rasterizes one shape.
 *
 * Recharts stays the right tool for the large interactive charts, which
 * update rarely and need its axis, legend, and brush behaviour.
 */
export const Sparkline = ({
  values,
  colorRgb,
  lineGraphType,
  fill,
  showScale,
  tooltip,
  range = [0, 100],
  className,
}: SparklineProps) => {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  const paths = buildSparklinePath({ values, range, lineGraphType });
  const hoveredValue = hoveredIndex == null ? null : values[hoveredIndex];

  const handlePointerMove = (
    event: React.PointerEvent<HTMLDivElement>,
  ): void => {
    const bounds = event.currentTarget.getBoundingClientRect();
    if (bounds.width === 0 || values.length === 0) {
      return;
    }

    const ratio = (event.clientX - bounds.left) / bounds.width;
    const index = Math.round(ratio * (values.length - 1));
    setHoveredIndex(Math.min(Math.max(index, 0), values.length - 1));
  };

  return (
    <div
      className={cn("relative h-full w-full", className)}
      {...(tooltip && {
        onPointerMove: handlePointerMove,
        onPointerLeave: () => setHoveredIndex(null),
      })}
    >
      <svg
        className="h-full w-full overflow-visible"
        viewBox={`0 0 ${sparklineViewBox.width} ${sparklineViewBox.height}`}
        preserveAspectRatio="none"
        role="presentation"
      >
        {showScale &&
          sparklineGridLines(gridTickCount).map((y) => (
            <line
              key={y}
              x1={0}
              x2={sparklineViewBox.width}
              y1={y}
              y2={y}
              className="stroke-border"
              strokeWidth={1}
              vectorEffect="non-scaling-stroke"
            />
          ))}
        {fill && <path d={paths.area} fill={`rgba(${colorRgb},0.3)`} />}
        <path
          d={paths.line}
          fill="none"
          stroke={`rgb(${colorRgb})`}
          strokeWidth={2}
          strokeLinejoin="round"
          strokeLinecap="round"
          vectorEffect="non-scaling-stroke"
        />
      </svg>

      {showScale && (
        <div
          className="pointer-events-none absolute inset-y-0 left-0 flex flex-col justify-between text-[10px] text-muted-foreground"
          aria-hidden
        >
          <span>{range[1]}</span>
          <span>{range[0]}</span>
        </div>
      )}

      {tooltip && hoveredValue != null && (
        <div className="pointer-events-none absolute top-1 right-1 rounded-md border border-border/50 bg-background/90 px-2 py-1 text-xs shadow-xl">
          <span className="text-muted-foreground">{tooltip.label}</span>{" "}
          <span className="font-medium font-mono text-foreground">
            {tooltip.format(hoveredValue)}
          </span>
        </div>
      )}
    </div>
  );
};
