import { useCallback, useEffect, useRef, useState } from "react";
import { type CoolingBandComparison, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Fetches the load-band comparison once per mount. Like the baseline delta,
 * this is a current-state fact gated by the same establishing/established
 * lifecycle, not a range query the period selector drives.
 */
export const useCoolingBandComparison = () => {
  const [data, setData] = useState<CoolingBandComparison | null>(null);
  const [hasError, setHasError] = useState(false);
  const requestIdRef = useRef(0);

  const load = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    // A rerun starts a fresh request: a failure from the previous run
    // must not stick to it.
    setHasError(false);

    try {
      const result = await commands.getCoolingBandComparison();
      if (isError(result)) {
        throw new Error(
          `Failed to fetch cooling band comparison: ${result.error}`,
        );
      }
      if (requestIdRef.current === requestId) {
        setData(result.data);
      }
    } catch (e) {
      console.error(e);
      // A stale request must not flip the state.
      if (requestIdRef.current === requestId) {
        setData(null);
        // A failure is not "still loading": consumers render a
        // load-failure line instead of keeping the skeleton forever.
        setHasError(true);
      }
    }
  }, []);

  useEffect(() => {
    void load();

    return () => {
      // Unmounting (or re-running) invalidates the in-flight request so a
      // late rejection cannot flip state after the view is gone.
      requestIdRef.current += 1;
    };
  }, [load]);

  const retry = useCallback(() => {
    void load();
  }, [load]);

  return { data, hasError, retry };
};
