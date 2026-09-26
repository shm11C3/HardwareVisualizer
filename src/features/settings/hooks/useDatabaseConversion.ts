import { useCallback, useEffect, useRef, useState } from "react";
import { commands, type DatabaseConversionState } from "@/rspc/bindings";
import { isError } from "@/types/result";

/** How often progress is re-read while a conversion is running. */
const ACTIVE_POLL_INTERVAL_MS = 800;

const initialState: DatabaseConversionState = { kind: "notSupported" };

/** The first step the backend driver reports - `start()`'s own optimistic
 * placeholder before a real progress read arrives. See `start()`. */
const OPTIMISTIC_STARTING_STATE: DatabaseConversionState = {
  kind: "converting",
  step: "preflight",
};

/**
 * Kinds `get_database_conversion_state` can still report for a moment
 * after a successful `start_database_conversion` resolves: the backend's
 * own point-in-time disk read that raced the driver's first progress
 * write (a resume/retry can take tens of ms to open the finalized file
 * and read its metadata before it does). `start()`'s own `refresh()` must
 * not let one of these overwrite the optimistic `converting` state it
 * just set - see `start()`.
 */
const isPreStartKind = (kind: DatabaseConversionState["kind"]) =>
  kind === "sqliteAuthoritative" || kind === "conversionRecoverable";

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
  // The initial state is indistinguishable from a real "not supported"
  // answer, so callers that must wait for the first read watch this.
  const [settled, setSettled] = useState(false);
  const previousKindRef = useRef<DatabaseConversionState["kind"] | null>(null);
  // True from a successful `start()` until `refresh()` observes a state
  // that is not a stale pre-start read - see `isPreStartKind` and
  // `start()`.
  const startPendingRef = useRef(false);
  // Bumped every time `start()` succeeds. Each `refresh()` call captures
  // this value *before* awaiting the command, and discards its own
  // response if a newer `start()` began while it was in flight - see
  // `refresh()`'s own comment for the race this closes.
  const startGenerationRef = useRef(0);

  const refresh = useCallback(async () => {
    const requestGeneration = startGenerationRef.current;
    try {
      const next = await commands.getDatabaseConversionState();
      if (requestGeneration < startGenerationRef.current) {
        // A newer `start()` began after this read was issued - e.g. React
        // Strict Mode's double-invoked mount effect leaves an earlier mount
        // read in flight, which can resolve *after* `start()`'s own refresh
        // already observed real progress. Without this check,
        // `startPendingRef` below would already be cleared by that newer
        // read, so this older one's stale (pre-start) kind would pass the
        // `isPreStartKind` guard and regress the UI. Whatever this read
        // reports is superseded either way, so it is discarded
        // unconditionally rather than re-checked against `isPreStartKind`.
        return;
      }
      if (startPendingRef.current && isPreStartKind(next.kind)) {
        // Racing a just-issued Start: the backend has not written its first
        // progress state yet. Keep showing the optimistic `converting`
        // state `start()` already set rather than regressing the UI to
        // what was true before Start was pressed - see #2245 and `start()`.
        return;
      }
      startPendingRef.current = false;
      if (
        previousKindRef.current === "converting" &&
        next.kind === "nativeAuthoritative"
      ) {
        setJustCompleted(true);
      }
      previousKindRef.current = next.kind;
      setState(next);
      return next;
    } finally {
      // A failed or discarded (superseded/stale) read still settles:
      // nothing should wait on it forever.
      setSettled(true);
    }
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
    // A successful Start means a conversion is now nominally in flight
    // from this call's perspective, even if the very first `refresh()`
    // below already observes `nativeAuthoritative` directly (a fast
    // completion that skipped over any `converting` poll this hook
    // happened to catch). Priming `previousKindRef` here, rather than
    // leaving it at whatever it was before Start, makes that refresh's
    // own converting-to-native check in `refresh()` fire correctly - and
    // is a no-op if the conversion is still genuinely running, since
    // `refresh()` would set the same value anyway.
    previousKindRef.current = "converting";
    // Set directly, rather than only relying on `refresh()` below to
    // observe it: the command can resolve before the backend's own first
    // progress write lands (a resume/retry's metadata read alone can take
    // tens of ms), so `refresh()` right below can still read the pre-start
    // state instead of `converting`. Setting it here is also what arms
    // the polling effect immediately, so a lagging first read never
    // leaves polling un-armed - see `isPreStartKind` and `refresh()`.
    startPendingRef.current = true;
    // Bumped before any read is issued below, so `refresh()` can tell an
    // in-flight read issued before this `start()` (e.g. a still-pending
    // mount read under React Strict Mode) apart from one issued after -
    // see `refresh()`'s own comment.
    startGenerationRef.current += 1;
    setState(OPTIMISTIC_STARTING_STATE);
    await refresh();
    return true;
  }, [refresh]);

  const recover = useCallback(async () => {
    setError(null);
    const result = await commands.rebuildNativeDatabaseFromSqlite();
    if (isError(result)) {
      setError(result.error);
      await refresh();
      return false;
    }
    previousKindRef.current = "converting";
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

  return {
    state,
    settled,
    error,
    start,
    recover,
    cancel,
    justCompleted,
    acknowledgeCompletion,
  };
};
