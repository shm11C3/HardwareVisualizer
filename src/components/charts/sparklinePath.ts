import {
  area,
  type CurveFactory,
  curveBasis,
  curveLinear,
  curveMonotoneX,
  curveStep,
} from "d3-shape";
import type { LineGraphType } from "@/rspc/bindings";

/**
 * The same curve factories Recharts resolves its `type` prop to, so a
 * Sparkline and a Recharts chart fed the same series draw the same shape.
 */
const lineGraphTypeToCurve: Record<LineGraphType, CurveFactory> = {
  default: curveMonotoneX,
  step: curveStep,
  linear: curveLinear,
  basis: curveBasis,
};

/**
 * Sparklines draw into this fixed coordinate box and are stretched to the
 * container by `preserveAspectRatio="none"`. Stretching is an affine
 * transform, and every curve above is affine-invariant, so the drawn shape
 * matches one computed at the container's real pixel size — which is what
 * lets the component skip measuring the element at all.
 */
export const sparklineViewBox = { width: 100, height: 100 } as const;

export type SparklinePathInput = {
  values: (number | null)[];
  /** Value range mapped onto the full height, as `[min, max]`. */
  range: [number, number];
  lineGraphType: LineGraphType;
};

const toPoints = ({
  values,
  range,
}: Omit<SparklinePathInput, "lineGraphType">) => {
  const [min, max] = range;
  const span = max - min || 1;
  const lastIndex = Math.max(values.length - 1, 1);

  return values.map((value, index) => ({
    x: (index / lastIndex) * sparklineViewBox.width,
    y: value == null ? 0 : (1 - (value - min) / span) * sparklineViewBox.height,
    defined: value != null,
  }));
};

/**
 * Build the stroke and fill paths for one series.
 *
 * Leading `null`s (a history buffer that is not full yet) and gaps become
 * breaks rather than a line to zero, matching how Recharts treats missing
 * points — an unavailable reading must not read as 0%.
 */
export const buildSparklinePath = ({
  values,
  range,
  lineGraphType,
}: SparklinePathInput): { line: string; area: string } => {
  const points = toPoints({ values, range });
  const curve = lineGraphTypeToCurve[lineGraphType];

  const areaShape = area<(typeof points)[number]>()
    .defined((point) => point.defined)
    .x((point) => point.x)
    .y0(sparklineViewBox.height)
    .y1((point) => point.y)
    .curve(curve);

  return {
    line: areaShape.lineY1()(points) ?? "",
    area: areaShape(points) ?? "",
  };
};

export type SparklineTick = {
  value: number;
  /** Position within the view box, top-down. */
  y: number;
};

/**
 * Evenly spaced scale ticks, highest first.
 *
 * `count` is honoured exactly rather than widened to a "nice" step: the
 * ranges these charts use divide evenly at the counts they ask for, and a
 * caller that asks for a tick per grid line must get the labels to match it.
 */
export const sparklineTicks = (
  range: [number, number],
  count: number,
): SparklineTick[] => {
  if (count < 2) {
    return [];
  }

  const [min, max] = range;

  return Array.from({ length: count }, (_, index) => {
    const ratio = index / (count - 1);

    return {
      value: Math.round(max - ratio * (max - min)),
      y: ratio * sparklineViewBox.height,
    };
  });
};
