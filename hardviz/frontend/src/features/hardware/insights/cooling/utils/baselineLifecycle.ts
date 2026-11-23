/**
 * The establishing/established shape shared by `CoolingBaselineState` and
 * `CoolingBandComparison` (see `src/rspc/bindings.ts`). Both gate their
 * content on the same idle-baseline lifecycle, so this file resolves the
 * empty state for either one without re-deriving the day counts Core
 * already computed.
 */
type EstablishingLifecycle = {
  status: "establishing";
  qualifyingDays: number;
  requiredDays: number;
};
type EstablishedLifecycle = { status: "established" };

export type BaselineLifecycleSource =
  | EstablishingLifecycle
  | EstablishedLifecycle;

export type BaselineLifecycleState =
  /** Not fetched yet. */
  | { kind: "loading" }
  /** Show the "establishing baseline (n / N days)" empty state. */
  | { kind: "establishing"; qualifyingDays: number; requiredDays: number }
  /** Baseline established; the zone's real content can render. */
  | { kind: "ready" };

export const resolveBaselineLifecycle = (
  source: BaselineLifecycleSource | null,
): BaselineLifecycleState => {
  if (source == null) {
    return { kind: "loading" };
  }

  if (source.status === "establishing") {
    return {
      kind: "establishing",
      qualifyingDays: source.qualifyingDays,
      requiredDays: source.requiredDays,
    };
  }

  return { kind: "ready" };
};
