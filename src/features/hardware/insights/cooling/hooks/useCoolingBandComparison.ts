import { useEffect, useRef, useState } from "react";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type CoolingBandComparison, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Fetches the load-band comparison once per mount. Like the baseline delta,
 * this is a current-state fact gated by the same establishing/established
 * lifecycle, not a range query the period selector drives.
 */
export const useCoolingBandComparison = () => {
  const [data, setData] = useState<CoolingBandComparison | null>(null);
  const { error } = useTauriDialog();
  const requestIdRef = useRef(0);

  useEffect(() => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    void (async () => {
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
        // A stale request must neither flip the state nor open a dialog.
        if (requestIdRef.current === requestId) {
          setData(null);
          void error(String(e));
        }
      }
    })();
  }, [error]);

  return { data };
};
