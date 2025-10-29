import { useCallback, useEffect, useState } from "react";

type BreakpointSize = "xs" | "sm" | "md" | "lg" | "xl" | "2xl";

const BREAKPOINT_ORDER: BreakpointSize[] = [
  "xs",
  "sm",
  "md",
  "lg",
  "xl",
  "2xl",
];

const getBreakpoint = (): BreakpointSize => {
  const width = globalThis.innerWidth;

  if (width >= 1536) return "2xl";
  if (width >= 1280) return "xl";
  if (width >= 1024) return "lg";
  if (width >= 768) return "md";
  if (width >= 640) return "sm";
  return "xs";
};

export const useWindowsSize = () => {
  const [breakpoint, setBreakpoint] = useState<BreakpointSize>(getBreakpoint());

  useEffect(() => {
    let timeoutId: NodeJS.Timeout;

    const handleResize = () => {
      clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        const newBreakpoint = getBreakpoint();

        setBreakpoint((prev) => {
          if (prev === newBreakpoint) return prev;
          return newBreakpoint;
        });
      }, 100);
    };

    globalThis.addEventListener("resize", handleResize);

    return () => {
      clearTimeout(timeoutId);
      globalThis.removeEventListener("resize", handleResize);
    };
  }, []);

  const isBreak = useCallback(
    (targetBreakpoint: BreakpointSize): boolean => {
      const currentIndex = BREAKPOINT_ORDER.indexOf(breakpoint);
      const targetIndex = BREAKPOINT_ORDER.indexOf(targetBreakpoint);

      return currentIndex >= targetIndex;
    },
    [breakpoint],
  );

  return { breakpoint, isBreak };
};
