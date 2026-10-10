import { atom, type Store } from "jotai";
import type { DatabaseConversionState } from "@/rspc/bindings";

/** `null` means not loaded from the Tauri Store yet. */
export const noticeShownAtom = atom<boolean | null>(null);

/**
 * Shared across every `useDatabaseConversion` mount (the app-root prompt
 * dialog and the Settings entry point each drive their own instance) - the
 * lifecycle kind most recently observed by *any* of them. A per-mount
 * `useRef` cannot detect a transition an unmounted observer missed: a
 * conversion started from Settings that finishes after the user navigates
 * away leaves the next Settings mount's own first `refresh()` reading
 * `nativeAuthoritative` with no local memory of ever having seen
 * `converting` (#2267). Sharing this value means whichever instance stays
 * mounted through the transition - normally the always-mounted app-root
 * prompt dialog, see `DatabaseConversionPromptDialog` - records it for every
 * other mount to read, and also lets every other mount's own polling effect
 * arm itself once *any* mount observes `converting`, not only its own.
 *
 * `null` means no mount has read a state yet this session, so a fresh
 * install's first-ever read (`nativeAuthoritative` with nothing converted)
 * is correctly never treated as a completion - see #2203.
 */
export const lastObservedDatabaseConversionKindAtom = atom<
  DatabaseConversionState["kind"] | null
>(null);

/**
 * Shared across every mount - see `lastObservedDatabaseConversionKindAtom`.
 * Set once by whichever mount's `refresh()` observes the `converting` ->
 * `nativeAuthoritative` transition, and cleared by `acknowledgeCompletion()`
 * once the one-time retention notice is dismissed.
 */
export const databaseConversionJustCompletedAtom = atom(false);

/**
 * Write-only atom: records a freshly observed kind from any
 * `useDatabaseConversion` mount. Reads and writes both shared atoms above
 * against the store's *current* value (via jotai's `get`/`set`) rather than
 * a value captured in a mount's own possibly-stale render closure, so the
 * converting -> nativeAuthoritative check is correct regardless of which
 * mount last rendered.
 */
export const recordObservedDatabaseConversionKindAtom = atom(
  null,
  (get, set, kind: DatabaseConversionState["kind"]) => {
    const previousKind = get(lastObservedDatabaseConversionKindAtom);
    if (previousKind === "converting" && kind === "nativeAuthoritative") {
      set(databaseConversionJustCompletedAtom, true);
    }
    set(lastObservedDatabaseConversionKindAtom, kind);
  },
);

/**
 * Guards the two atoms above against an out-of-order write from across
 * mounts. Every mount's own local `startGenerationRef`/`requestGeneration`
 * check only orders that *one instance's* requests against its own later
 * ones; it says nothing about a *different* instance's request. Without a
 * cross-mount guard, the app-root prompt dialog's own initial `refresh()` -
 * issued before any conversion exists, so it is a perfectly ordinary read -
 * can still be slow enough to resolve with `sqliteAuthoritative` *after*
 * Settings has already started a conversion and recorded `converting` (or
 * further progress); an unconditional write would then clobber the shared
 * state right back to `sqliteAuthoritative`, undoing the observation this
 * fix exists to preserve.
 *
 * A plain counter pair is enough here - unlike the atoms above, its value is
 * only ever compared, never rendered. It is owned per Jotai store, so the
 * mounts that share an atom also share the counters while independent stores
 * (tests, future windows) never order each other's reads. `reserve()` hands out the
 * next sequence number when a read is issued (or a definite state change,
 * like `start()`'s own priming, happens); the caller passes that same
 * number back once the read resolves (or immediately, for a synchronous
 * change), and it is only allowed to write if no *later-issued* sequence
 * number has already written - i.e. it is not stale relative to every write
 * that has happened since it was reserved, regardless of resolution order.
 */
type DatabaseConversionSequences = {
  latest: number;
  committed: number;
};

const databaseConversionSequences = new WeakMap<
  Store,
  DatabaseConversionSequences
>();

const getDatabaseConversionSequences = (
  store: Store,
): DatabaseConversionSequences => {
  let sequences = databaseConversionSequences.get(store);
  if (sequences === undefined) {
    sequences = { latest: 0, committed: 0 };
    databaseConversionSequences.set(store, sequences);
  }
  return sequences;
};

export const reserveDatabaseConversionSequence = (store: Store) =>
  ++getDatabaseConversionSequences(store).latest;

/** See `reserveDatabaseConversionSequence`. Returns whether `sequence` was
 * still current enough to write, and records that a write for it happened
 * either way (a lower/equal sequence than one that already wrote is always
 * stale, whether or not it goes on to write anything of its own next). */
export const shouldRecordDatabaseConversionSequence = (
  store: Store,
  sequence: number,
) => {
  const sequences = getDatabaseConversionSequences(store);
  if (sequence < sequences.committed) {
    return false;
  }
  sequences.committed = sequence;
  return true;
};

/** For a definite, synchronous state change (`start()`/`recover()`'s own
 * priming) rather than a read racing anything else: reserves and commits a
 * fresh sequence number in one step, so it always supersedes any
 * earlier-issued, still in-flight read from another mount - see
 * `shouldRecordDatabaseConversionSequence`. */
export const commitFreshDatabaseConversionSequence = (store: Store) => {
  shouldRecordDatabaseConversionSequence(
    store,
    reserveDatabaseConversionSequence(store),
  );
};
