import * as SliderPrimitive from "@radix-ui/react-slider";
import * as React from "react";
import { cn } from "@/lib/utils";

// Simple ID generation (no external dependencies)
const makeId = (() => {
  let n = 0;
  return () => `thumb-${n++}`;
})();

function reorderIdsByNearest(
  prevValues: number[],
  nextValues: number[],
  prevIds: string[],
) {
  // Greedily match "next values" to "previous values" by minimum distance (1-to-1)
  const usedPrev = new Set<number>();
  const mapping: string[] = new Array(nextValues.length);

  for (let i = 0; i < nextValues.length; i++) {
    let bestJ = -1;
    let bestDist = Number.POSITIVE_INFINITY;
    for (let j = 0; j < prevValues.length; j++) {
      if (usedPrev.has(j)) continue;
      const d = Math.abs(nextValues[i] - prevValues[j]);
      if (d < bestDist) {
        bestDist = d;
        bestJ = j;
      }
    }
    usedPrev.add(bestJ);
    mapping[i] = prevIds[bestJ];
  }
  return mapping;
}

type SliderProps = React.ComponentPropsWithoutRef<typeof SliderPrimitive.Root>;

export const Slider = React.forwardRef<
  React.ComponentRef<typeof SliderPrimitive.Root>,
  SliderProps
>(({ className, value, onValueChange, ...props }, ref) => {
  // Maintain persistent IDs according to the length of values
  const [ids, setIds] = React.useState<string[]>(() =>
    Array.from({ length: value?.length ?? 1 }, () => makeId()),
  );
  const prevValuesRef = React.useRef<number[] | null>(value ?? null);

  // Adjust IDs when thumb count changes (add to end / truncate from end)
  React.useEffect(() => {
    const len = value?.length ?? 1;
    setIds((prev) => {
      if (prev.length === len) return prev;
      if (prev.length < len) {
        return [
          ...prev,
          ...Array.from({ length: len - prev.length }, () => makeId()),
        ];
      }
      return prev.slice(0, len);
    });
  }, [value?.length]);

  // When values are updated, estimate which ID corresponds to which position and reorder
  React.useEffect(() => {
    if (!value) return;
    const prev = prevValuesRef.current;
    if (
      prev &&
      prev.length === value.length &&
      prev.some((v, i) => v !== value[i])
    ) {
      setIds((prevIds) => reorderIdsByNearest(prev, value, prevIds));
    }
    prevValuesRef.current = value.slice();
  }, [value]);

  return (
    <SliderPrimitive.Root
      ref={ref}
      className={cn(
        "relative flex w-full touch-none select-none items-center",
        className,
      )}
      value={value}
      onValueChange={onValueChange}
      {...props}
    >
      <SliderPrimitive.Track className="relative h-2 w-full grow overflow-hidden rounded-full bg-secondary">
        <SliderPrimitive.Range className="absolute h-full bg-primary" />
      </SliderPrimitive.Track>

      {(value ?? [0]).map((_, i) => (
        <SliderPrimitive.Thumb
          key={ids[i]}
          className="block h-5 w-5 rounded-full border-2 border-primary bg-background ring-offset-background transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:pointer-events-none disabled:opacity-50"
        />
      ))}
    </SliderPrimitive.Root>
  );
});
Slider.displayName = SliderPrimitive.Root.displayName;
