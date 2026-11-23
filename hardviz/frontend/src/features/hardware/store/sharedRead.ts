import { type Atom, atom, type WritableAtom } from "jotai";
import { atomWithRefresh, unwrap } from "jotai/utils";

/**
 * What a panel sees of a read that more than one panel shares.
 *
 * `params` are the exact values the request was made with. A panel never
 * compares them itself: this module already reads every state whose params
 * are not the current ones as `loading`, so a `ready` value is always for
 * the period on screen.
 */
export type SharedReadState<P, D> =
  | { kind: "loading" }
  | { kind: "ready"; params: P; data: D }
  | { kind: "failed"; params: P };

/**
 * Writes a shared read accepts. Callers only ever write `"refresh"`; the
 * other two are the atom's own mount lifecycle.
 */
export type SharedReadAction = "refresh" | "enter" | "leave";

export type SharedReadAtom<P, D> = WritableAtom<
  SharedReadState<P, D>,
  [action: SharedReadAction],
  void
>;

type Settled<P, D> =
  | { kind: "ready"; params: P; data: D }
  | { kind: "failed"; params: P }
  // The read had nothing to ask (`params` was null).
  | { kind: "idle" };

type Pending<P, D> = { kind: "pending"; previous: Settled<P, D> | undefined };

const LOADING = { kind: "loading" } as const;

/**
 * Build the atom for one read that several panels share.
 *
 * - `params` is the request's input, derived from other atoms. `null` means
 *   "nothing to ask" (for example the daily trend while the period routes to
 *   the archive): the read neither fetches nor claims data. Params are
 *   compared with `Object.is`, so they must be primitives.
 * - `fetch` rejects on failure. The atom itself never rejects: a failure is
 *   a `failed` state, distinct from an empty `ready` one.
 *
 * The returned atom reads as a {@link SharedReadState} and writes as a
 * refresh. Its lifetime follows the panels that subscribe to it:
 *
 * - Subscribers share one request. A refresh while one is in flight replaces
 *   it and jotai drops the superseded promise, so no request-id guard is
 *   needed.
 * - Changing `params` reads as `loading` until the new request lands, so a
 *   panel never shows the previous period's data under the new period's
 *   label. Refreshing with the same params keeps the last data visible
 *   until the new one lands; a retry from a failure reads as `loading`.
 * - Nothing fetches while no panel is subscribed, and nothing in here
 *   schedules a refetch: a periodic refresh is a hook that writes this atom.
 * - A settled read does not outlive the visit that fetched it: the first
 *   subscriber of a later visit refetches it once. A request still in flight
 *   when the panels left is reused instead of duplicated.
 */
export const createSharedRead = <P, D>({
  params,
  fetch,
}: {
  params: Atom<P | null>;
  fetch: (params: P) => Promise<D>;
}): SharedReadAtom<P, D> => {
  const requestAtom = atomWithRefresh(async (get): Promise<Settled<P, D>> => {
    const current = get(params);
    if (current == null) {
      return { kind: "idle" };
    }
    try {
      return { kind: "ready", params: current, data: await fetch(current) };
    } catch (error) {
      console.error(error);
      return { kind: "failed", params: current };
    }
  });

  // `unwrap`'s fallback only receives the previous value, not the params the
  // new request is for, so it just carries the previous value out. The
  // params rule is applied below, where both are readable.
  const settledAtom = unwrap(
    requestAtom,
    (previous?: Settled<P, D>): Pending<P, D> => ({
      kind: "pending",
      previous,
    }),
  );

  // True once a visit (a stretch with at least one subscriber) has ended.
  const leftAtom = atom(false);

  const readAtom = atom(
    (get): SharedReadState<P, D> => {
      const current = get(params);
      const latest = get(settledAtom);
      const settled = latest.kind === "pending" ? latest.previous : latest;
      if (
        current == null ||
        settled == null ||
        settled.kind === "idle" ||
        !Object.is(settled.params, current)
      ) {
        return LOADING;
      }
      // A retry from a failure reads as loading: keeping the failure line on
      // screen would give "Try again" no visible response. Data, by contrast,
      // stays visible while its refresh is in flight.
      if (latest.kind === "pending" && settled.kind === "failed") {
        return LOADING;
      }
      return settled;
    },
    (get, set, action: SharedReadAction) => {
      if (action === "leave") {
        set(leftAtom, true);
        return;
      }
      if (action === "enter") {
        // The very first visit already started its request through the read.
        if (!get(leftAtom)) {
          return;
        }
        set(leftAtom, false);
        if (get(params) == null || get(settledAtom).kind === "pending") {
          return;
        }
      }
      set(requestAtom);
    },
  );

  readAtom.onMount = (setAtom) => {
    setAtom("enter");
    return () => {
      setAtom("leave");
    };
  };

  return readAtom;
};
