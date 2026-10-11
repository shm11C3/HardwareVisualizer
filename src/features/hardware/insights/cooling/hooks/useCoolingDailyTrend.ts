import { useCallback, useEffect, useRef, useState } from "react";
import { type CoolingDailyTrendPoint, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Fetches the daily cooling rollup for the 90d/1y routes
 * (`resolveCoolingPeriodRoute`). `days: null` means the current period
 * routes to the archive bucket query instead, so this simply idles.
 *
 * `data` distinguishes three states the strip must not conflate:
 * `null` while nothing has been established yet (idle, in flight, or
 * failed - see `hasError`), `[]` only when the backend really answered
 * with an empty window, and a non-empty array otherwise. A failed fetch
 * never masquerades as an empty period.
 */
export const useCoolingDailyTrend = (days: 90 | 365 | null) => {
  const [data, setData] = useState<CoolingDailyTrendPoint[] | null>(null);
  const [hasError, setHasError] = useState(false);
  const requestIdRef = useRef(0);

  const load = useCallback(async () => {
    if (days == null) {
      requestIdRef.current += 1;
      setData(null);
      setHasError(false);
      return;
    }

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    setData(null);
    setHasError(false);

    try {
      const result = await commands.getCoolingTrend(days);
      if (isError(result)) {
        throw new Error(`Failed to fetch cooling trend: ${result.error}`);
      }
      if (requestIdRef.current === requestId) {
        setData(result.data);
      }
    } catch (e) {
      console.error(e);
      // A stale request must not flip the state:
      // the user has already moved on to a newer period.
      if (requestIdRef.current === requestId) {
        setHasError(true);
      }
    }
  }, [days]);

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
