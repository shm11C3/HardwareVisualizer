import { useMemo } from "react";

/**
 * Minimal short-window trend line. Null samples break the polyline so
 * unavailable readings stay visible as gaps instead of interpolating.
 */
export const Sparkline = ({
  values,
  color,
  className = "h-9 w-full overflow-visible",
  showBaseline = true,
}: {
  values: (number | null)[];
  color: string;
  className?: string;
  /**
   * The baseline sits at the bottom of the SVG box. Hide it where a real
   * divider already separates rows, so the two do not read as one broken line.
   */
  showBaseline?: boolean;
}) => {
  const segments = useMemo(() => {
    const width = 180;
    const height = 48;
    const denominator = Math.max(values.length - 1, 1);
    const nextSegments: Array<{ startIndex: number; points: string[] }> = [];

    values.forEach((value, index) => {
      if (value == null) {
        return;
      }

      const point = `${(index / denominator) * width},${
        height - (Math.min(100, Math.max(0, value)) / 100) * height
      }`;
      const previousValue = values[index - 1];
      if (index === 0 || previousValue == null) {
        nextSegments.push({ startIndex: index, points: [point] });
        return;
      }

      nextSegments.at(-1)?.points.push(point);
    });

    return nextSegments.map(({ startIndex, points }) => ({
      startIndex,
      points: points.join(" "),
    }));
  }, [values]);

  return (
    <svg
      viewBox="0 0 180 48"
      preserveAspectRatio="none"
      aria-hidden="true"
      className={className}
    >
      {showBaseline && (
        <path
          d="M0 47.5H180"
          stroke="currentColor"
          strokeOpacity="0.12"
          vectorEffect="non-scaling-stroke"
        />
      )}
      {segments.map(({ startIndex, points }) => (
        <polyline
          key={startIndex}
          points={points}
          fill="none"
          stroke={color}
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
          vectorEffect="non-scaling-stroke"
        />
      ))}
    </svg>
  );
};
