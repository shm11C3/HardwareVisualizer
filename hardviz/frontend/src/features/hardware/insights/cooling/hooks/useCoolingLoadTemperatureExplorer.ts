import { useCallback, useEffect, useRef, useState } from "react";
import { type CoolingLoadTemperatureExplorer, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Fetches the load-vs-temperature Explorer for a trailing window of
 * `recentDays`.
 *
 * `recentDays: null` means "do not ask": the Explorer is collapsed, and a
 * folded-away secondary analysis must not cost a query (DP-04). Expanding
 * it, or changing the preset, starts a fresh request.
 */
export const useCoolingLoadTemperatureExplorer = (
  recentDays: number | null,
) => {
  const [data, setData] = useState<CoolingLoadTemperatureExplorer | null>(null);
  const [hasError, setHasError] = useState(false);
  const requestIdRef = useRef(0);

  const load = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    // A rerun starts a fresh request: neither a failure nor the windows
    // fetched for the *previous* `recentDays` may stick to it. Dropping
    // the data returns the panel to its loading state, so a preset change
    // never leaves the old period's scatter on screen labelled as the new
    // one.
    setHasError(false);
    setData(null);

    if (recentDays == null) {
      // Collapsed: issue no request at all.
      return;
    }

    try {
      const result =
        await commands.getCoolingLoadTemperatureExplorer(recentDays);
      if (isError(result)) {
        throw new Error(
          `Failed to fetch cooling load-temperature explorer: ${result.error}`,
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
  }, [recentDays]);

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
