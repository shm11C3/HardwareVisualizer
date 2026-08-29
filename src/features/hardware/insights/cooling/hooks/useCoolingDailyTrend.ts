import { useEffect, useRef, useState } from "react";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type CoolingDailyTrendPoint, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Fetches the daily cooling rollup for the 90d/1y routes
 * (`resolveCoolingPeriodRoute`). `days: null` means the current period
 * routes to the archive bucket query instead, so this simply idles.
 */
export const useCoolingDailyTrend = (days: 90 | 365 | null) => {
  const [data, setData] = useState<CoolingDailyTrendPoint[] | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const { error } = useTauriDialog();
  const requestIdRef = useRef(0);

  useEffect(() => {
    if (days == null) {
      requestIdRef.current += 1;
      setData(null);
      setIsLoading(false);
      return;
    }

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    setIsLoading(true);

    void (async () => {
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
        if (requestIdRef.current === requestId) {
          setData([]);
        }
        void error(String(e));
      } finally {
        if (requestIdRef.current === requestId) {
          setIsLoading(false);
        }
      }
    })();
  }, [days, error]);

  return { data, isLoading };
};
