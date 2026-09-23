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

  // Initial fetch, once per mount.
  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Poll while (and only while) a conversion is actually running. Keyed on
  // `state.kind` rather than armed once at mount: `start()` and `cancel()`
  // both change `state.kind` through the same `refresh()`/`setState` path
  // this effect watches, so a transition either of them causes re-arms (or
  // tears down) this interval exactly like the initial mount does. Arming
  // polling only inside `start()` itself would miss a `converting` state
  // this hook observes for any other reason (e.g. a second tab, or a
  // conversion someone else already started), and would leave the UI
  // stuck on the first observed step once `start()`'s own `refresh()`
  // returns - see #2220's review discussion.
  useEffect(() => {
    if (state.kind !== "converting") {
      return;
    }
    const timer = setInterval(() => {
      void refresh();
    }, ACTIVE_POLL_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [state.kind, refresh]);

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
