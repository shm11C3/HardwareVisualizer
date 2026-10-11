import { useCallback, useEffect, useRef, useState } from "react";
import { type CoolingBaselineDelta, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Fetches the idle-baseline delta card once per mount. Unlike the archive
 * charts, this does not depend on the selected Cooling Insight period: the
 * baseline lifecycle (establishing/established) and its daily-delta series
 * are Core-owned facts about the current state, not a queryable range.
 */
export const useCoolingBaselineDelta = () => {
  const [data, setData] = useState<CoolingBaselineDelta | null>(null);
  const [hasError, setHasError] = useState(false);
  const requestIdRef = useRef(0);

  const load = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    // A rerun starts a fresh request: a failure from the previous run
    // must not stick to it.
    setHasError(false);

    try {
      const result = await commands.getCoolingBaselineDelta();
      if (isError(result)) {
        throw new Error(
          `Failed to fetch cooling baseline delta: ${result.error}`,
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
