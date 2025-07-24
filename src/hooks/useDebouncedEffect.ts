import { useEffect } from "react";

export const useDebouncedEffect = (
  effect: () => void,
  deps: any[],
  delay: number,
) => {
  useEffect(() => {
    const handler = setTimeout(() => {
      effect();
    }, delay);

    return () => {
      clearTimeout(handler);
    };
  }, [...deps, delay, effect]);
};
