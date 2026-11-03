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
        "relative flex w-full touch-none items-center select-none",
        className,
      )}
      {...(value !== undefined && { value })}
      {...(onValueChange !== undefined && { onValueChange })}
      {...props}
    >
      <SliderPrimitive.Track className="bg-secondary relative h-2 w-full grow overflow-hidden rounded-full">
        <SliderPrimitive.Range className="bg-primary absolute h-full" />
      </SliderPrimitive.Track>

      {(value ?? [0]).map((_, i) => (
        <SliderPrimitive.Thumb
          key={ids[i]}
          data-slot="slider-thumb"
          className="border-primary bg-background ring-offset-background focus-visible:ring-ring block h-5 w-5 rounded-full border-2 transition-colors focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
        />
      ))}
    </SliderPrimitive.Root>
  );
});
Slider.displayName = SliderPrimitive.Root.displayName;
