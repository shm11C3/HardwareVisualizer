import { useEffect, useRef, useState } from "react";
import { useTauriDialog } from "@/hooks/useTauriDialog";
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
  const { error } = useTauriDialog();
  const requestIdRef = useRef(0);

  useEffect(() => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    void (async () => {
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
