import { useEffect, useRef, useState } from "react";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import {
  type CoolingCovariateComparison,
  type CoolingLoadBand,
  commands,
} from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Fetches the co-variate comparison (#2068) for `band` once per mount.
 * Like the band comparison, it is a current-state fact gated by the
 * Thermal Delta Baseline's own establishing/established lifecycle, not a
 * range query the period selector drives.
 *
 * `band: null` idles - the caller already knows the machine has no ambient
 * source, so the comparison could only answer "establishing" and the
 * round trip would buy nothing.
 */
export const useCoolingCovariateComparison = (band: CoolingLoadBand | null) => {
  const [data, setData] = useState<CoolingCovariateComparison | null>(null);
  const [hasError, setHasError] = useState(false);
  const { error } = useTauriDialog();
  const requestIdRef = useRef(0);

  useEffect(() => {
    if (band == null) {
      requestIdRef.current += 1;
      setData(null);
      setHasError(false);
      return;
    }

    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    // A rerun starts a fresh request: a failure from the previous run
    // must not stick to it.
    setHasError(false);

    void (async () => {
      try {
        const result = await commands.getCoolingCovariateComparison(band);
        if (isError(result)) {
          throw new Error(
            `Failed to fetch cooling covariate comparison: ${result.error}`,
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
          // A failure is not "still loading": consumers render a
          // load-failure line instead of keeping the skeleton forever.
          setHasError(true);
          void error(String(e));
        }
      }
    })();

    return () => {
      // Unmounting invalidates the in-flight request so a late rejection
      // can neither flip state nor open a dialog after the view is gone.
      requestIdRef.current += 1;
    };
  }, [band, error]);

  return { data, hasError };
};
