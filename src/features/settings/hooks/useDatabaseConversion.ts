import { useCallback, useEffect, useRef, useState } from "react";
import { commands, type DatabaseConversionState } from "@/rspc/bindings";
import { isError } from "@/types/result";

/** How often progress is re-read while a conversion is running. */
const ACTIVE_POLL_INTERVAL_MS = 800;

const initialState: DatabaseConversionState = { kind: "notSupported" };

/**
 * Reads and drives the #2136 explicit database conversion flow.
 *
 * Progress reaches this hook by polling `get_database_conversion_state`,
 * which reads the App's own lifecycle owner - not a second source of
 * truth. Polling only runs at `ACTIVE_POLL_INTERVAL_MS` while a
 * conversion is actually running (`state.kind === "converting"`);
 * otherwise it fetches once per mount/action, so a Settings visit that
 * never starts a conversion costs one command call rather than a
 * standing interval (collection cost follows visible value).
 */
export const useDatabaseConversion = () => {
  const [state, setState] = useState<DatabaseConversionState>(initialState);
  const [error, setError] = useState<string | null>(null);
  const [justCompleted, setJustCompleted] = useState(false);
  const previousKindRef = useRef<DatabaseConversionState["kind"] | null>(null);

  const refresh = useCallback(async () => {
    const next = await commands.getDatabaseConversionState();
    if (
      previousKindRef.current === "converting" &&
      next.kind === "nativeAuthoritative"
    ) {
      setJustCompleted(true);
    }
    previousKindRef.current = next.kind;
    setState(next);
    return next;
  }, []);

  useEffect(() => {
    let cancelled = false;
    let timer: ReturnType<typeof setInterval> | null = null;

    const scheduleIfConverting = (current: DatabaseConversionState) => {
      if (timer) {
        clearInterval(timer);
        timer = null;
      }
      if (current.kind === "converting") {
        timer = setInterval(() => {
          void refresh().then((next) => {
            if (!cancelled) {
              scheduleIfConverting(next);
            }
          });
        }, ACTIVE_POLL_INTERVAL_MS);
      }
    };

    void refresh().then((next) => {
      if (!cancelled) {
        scheduleIfConverting(next);
      }
    });

    return () => {
      cancelled = true;
      if (timer) {
        clearInterval(timer);
      }
    };
  }, [refresh]);

  const start = useCallback(async () => {
    setError(null);
    const result = await commands.startDatabaseConversion();
    if (isError(result)) {
      setError(result.error);
      return false;
    }
    await refresh();
    return true;
  }, [refresh]);

  const cancel = useCallback(async () => {
    const result = await commands.cancelDatabaseConversion();
    if (isError(result)) {
      setError(result.error);
      return false;
    }
    await refresh();
    return true;
  }, [refresh]);

  const acknowledgeCompletion = useCallback(() => {
    setJustCompleted(false);
  }, []);

  return { state, error, start, cancel, justCompleted, acknowledgeCompletion };
};
