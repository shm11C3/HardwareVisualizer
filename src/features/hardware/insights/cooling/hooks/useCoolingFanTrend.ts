import { useCallback, useEffect, useRef, useState } from "react";
import { type CoolingFanTrend, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Fetches the daily fan rollup for the 90d/1y routes
 * (`resolveCoolingPeriodRoute`). `days: null` means the current period
 * routes to the archive bucket query instead, so this simply idles.
 *
 * A separate fetch from `useCoolingDailyTrend` because the fan rollup is a
 * separate Core query: it answers one series per fan rather than one row
 * per day, and a machine with no readable fan legitimately answers with
 * nothing while the CPU trend still has every day.
 *
 * `data` stays `null` while nothing is established yet (idle, in flight,
 * or failed - see `hasError`), so a failed fetch never masquerades as a
 * machine without fans. Core answers with the summarized series *and*
 * whether the one-minute fan archive holds anything, because an empty
 * series alone cannot tell "no readable fan" from "the rollup has not
 * summarized a completed day yet".
 */
export const useCoolingFanTrend = (days: 90 | 365 | null) => {
  const [data, setData] = useState<CoolingFanTrend | null>(null);
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
      const result = await commands.getCoolingFanTrend(days);
      if (isError(result)) {
        throw new Error(`Failed to fetch cooling fan trend: ${result.error}`);
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
