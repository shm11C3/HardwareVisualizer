import { useCallback, useEffect, useRef, useState } from "react";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Archived GPU names feed the Insights GPU tabs. A failed read leaves those
 * tabs out, so it is a one-panel read failure: the caller renders `hasError`
 * with `retry` where the tabs would appear (see "Failure Reporting" in the
 * frontend architecture doc), and an empty `gpuNames` never means "no GPU".
 */
export const useGpuNames = () => {
  const [gpuNames, setGpuNames] = useState<string[]>([]);
  const [hasError, setHasError] = useState(false);
  // The first read finished, successfully or not: until then a stored GPU tab
  // cannot be resolved yet.
  const [settled, setSettled] = useState(false);
  const requestIdRef = useRef(0);

  const load = useCallback(async () => {
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    try {
      const result = await commands.getGpuArchiveNames();
      if (isError(result)) {
        throw new Error(`Failed to fetch archived GPU names: ${result.error}`);
      }
      if (requestIdRef.current === requestId) {
        setGpuNames(result.data);
        setHasError(false);
        setSettled(true);
      }
    } catch (err) {
      console.error(err);
      // A stale request must not flip the state.
      if (requestIdRef.current === requestId) {
        setGpuNames([]);
        setHasError(true);
        setSettled(true);
      }
    }
  }, []);

  useEffect(() => {
    void load();

    return () => {
      requestIdRef.current += 1;
    };
  }, [load]);

  const retry = useCallback(() => {
    void load();
  }, [load]);

  return { gpuNames, hasError, settled, retry };
};
