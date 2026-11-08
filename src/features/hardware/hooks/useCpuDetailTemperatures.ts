import { useEffect, useRef, useState } from "react";
import { type CpuDetailTemperatures, commands } from "@/rspc/bindings";

const CPU_TEMPERATURE_POLL_MS = 3000;

export const useCpuDetailTemperatures = () => {
  const [sample, setSample] = useState<CpuDetailTemperatures | null>(null);
  const [hasError, setHasError] = useState(false);
  const refresh = useRef<() => void>(() => {});

  useEffect(() => {
    let generation = 0;
    let active = true;
    let inFlight = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const poll = async () => {
      clearTimeout(timer);
      if (!active || document.hidden || inFlight) {
        return;
      }

      inFlight = true;
      const requestGeneration = generation;
      const isCurrent = () =>
        active && !document.hidden && requestGeneration === generation;

      try {
        const result = await commands.getCpuDetailTemperatures();
        if (result.status === "error") {
          throw new Error(result.error);
        }
        if (isCurrent()) {
          setSample(result.data);
          setHasError(false);
        }
      } catch (error) {
        if (isCurrent()) {
          setSample(null);
          setHasError(true);
          console.error("Failed to fetch CPU detail temperatures:", error);
        }
      } finally {
        inFlight = false;
        if (active && !document.hidden) {
          timer = setTimeout(
            poll,
            requestGeneration === generation ? CPU_TEMPERATURE_POLL_MS : 0,
          );
        }
      }
    };

    const onVisibilityChange = () => {
      generation += 1;
      if (document.hidden) {
        clearTimeout(timer);
        setSample(null);
        setHasError(false);
      } else {
        void poll();
      }
    };

    refresh.current = () => void poll();
    timer = setTimeout(poll, 0);
    document.addEventListener("visibilitychange", onVisibilityChange);
    return () => {
      active = false;
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", onVisibilityChange);
    };
  }, []);

  return { sample, hasError, retry: () => refresh.current() };
};
