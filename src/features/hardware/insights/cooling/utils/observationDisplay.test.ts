import { describe, expect, it } from "vitest";
import type { CoolingAmbientAdjustedBaselineDelta } from "@/rspc/bindings";
import {
  daysInclusive,
  resolveAmbientAdjustedDisplay,
  resolveObservationDisplay,
} from "./observationDisplay";

describe("resolveObservationDisplay", () => {
  it("withholds a delta for notComparable, since Core reports none", () => {
    expect(resolveObservationDisplay("notComparable", null, 0, "C")).toEqual({
      kind: "notComparable",
      tone: "muted",
    });
  });

  it("reports withinRange with the delta unchanged in Celsius", () => {
    expect(resolveObservationDisplay("withinRange", 1.5, 0, "C")).toEqual({
      kind: "withinRange",
      tone: "positive",
      delta: 1.5,
    });
  });

  it("keeps a missing delta missing instead of coercing it to zero", () => {
    // A null delta must never render as a measured ±0.0° (DP-02).
    expect(resolveObservationDisplay("withinRange", null, 0, "F")).toEqual({
      kind: "withinRange",
      tone: "positive",
      delta: null,
    });
    expect(
      resolveObservationDisplay("sustainedMildRise", null, 3, "C"),
    ).toEqual({
      kind: "sustainedMildRise",
      tone: "mild",
      delta: null,
      sustainedDays: 3,
    });
  });

  it("converts the delta to Fahrenheit as a span, not a point conversion", () => {
    const result = resolveObservationDisplay("withinRange", 5, 0, "F");
    expect(result.kind).toBe("withinRange");
    expect((result as { delta: number }).delta).toBeCloseTo(9);
  });

  it("surfaces Core's sustained-day count for a mild rise", () => {
    expect(resolveObservationDisplay("sustainedMildRise", 6.2, 3, "C")).toEqual(
      { kind: "sustainedMildRise", tone: "mild", delta: 6.2, sustainedDays: 3 },
    );
  });

  it("distinguishes a large rise from a mild one via tone", () => {
    expect(
      resolveObservationDisplay("sustainedLargeRise", 11.5, 3, "C"),
    ).toEqual({
      kind: "sustainedLargeRise",
      tone: "large",
      delta: 11.5,
      sustainedDays: 3,
    });
  });
});

const ambientAdjusted = (
  overrides: Partial<CoolingAmbientAdjustedBaselineDelta> = {},
): CoolingAmbientAdjustedBaselineDelta => ({
  baseline: { status: "establishing", qualifyingDays: 0, requiredDays: 7 },
  recent: { deltaAvg: null, sampleMinutes: 0 },
  delta: null,
  comparable: false,
  ...overrides,
});

const established = {
  status: "established",
  deltaTemperatureAvg: 26,
  windowStartDate: "2025-12-01",
  windowEndDate: "2025-12-14",
  sampleMinutes: 9_800,
} as const;

describe("resolveAmbientAdjustedDisplay", () => {
  it("stays hidden on a machine with no environmental sensor", () => {
    // Core always sends this branch - an establishing ΔT baseline at zero
    // qualifying days is exactly how "no ambient sensor" reports itself.
    // Rendering its progress would put an ambient line on every machine.
    expect(resolveAmbientAdjustedDisplay(ambientAdjusted(), "C")).toEqual({
      kind: "hidden",
    });
  });

  it("reports ΔT establishment progress once paired days exist", () => {
    expect(
      resolveAmbientAdjustedDisplay(
        ambientAdjusted({
          baseline: {
            status: "establishing",
            qualifyingDays: 3,
            requiredDays: 7,
          },
        }),
        "C",
      ),
    ).toEqual({ kind: "establishing", qualifyingDays: 3, requiredDays: 7 });
  });

  it("stays hidden while the recent window is too thin to compare", () => {
    // "Insufficient coverage" must leave the strip exactly as it reads
    // without ambient data, not add a not-comparable line of its own.
    expect(
      resolveAmbientAdjustedDisplay(
        ambientAdjusted({
          baseline: established,
          recent: { deltaAvg: null, sampleMinutes: 12 },
        }),
        "C",
      ),
    ).toEqual({ kind: "hidden" });
  });

  it("reports the ambient-adjusted delta with the ΔT baseline's own window", () => {
    // The ΔT baseline establishes over its own days, so its window is
    // routinely a different range than the absolute baseline's.
    expect(
      resolveAmbientAdjustedDisplay(
        ambientAdjusted({
          baseline: established,
          recent: { deltaAvg: 30.8, sampleMinutes: 6_100 },
          delta: 4.8,
          comparable: true,
        }),
        "C",
      ),
    ).toEqual({
      kind: "comparable",
      delta: 4.8,
      windowStartDate: "2025-12-01",
      windowEndDate: "2025-12-14",
    });
  });

  it("converts the ambient-adjusted delta as a span, not a point", () => {
    const result = resolveAmbientAdjustedDisplay(
      ambientAdjusted({
        baseline: established,
        recent: { deltaAvg: 30.8, sampleMinutes: 6_100 },
        delta: 5,
        comparable: true,
      }),
      "F",
    );
    expect(result.kind).toBe("comparable");
    expect((result as { delta: number }).delta).toBeCloseTo(9);
  });

  it("stays hidden when Core reports comparable without a delta", () => {
    // Core's contract says `delta` is non-null whenever `comparable`; a
    // response that contradicts it must not render as a fabricated 0.
    expect(
      resolveAmbientAdjustedDisplay(
        ambientAdjusted({
          baseline: established,
          recent: { deltaAvg: 30.8, sampleMinutes: 6_100 },
          delta: null,
          comparable: true,
        }),
        "C",
      ),
    ).toEqual({ kind: "hidden" });
  });

  it("stays hidden when the response carries no ambient reading at all", () => {
    expect(resolveAmbientAdjustedDisplay(null, "C")).toEqual({
      kind: "hidden",
    });
  });
});

describe("daysInclusive", () => {
  it("counts a single day as 1", () => {
    expect(daysInclusive("2026-01-15", "2026-01-15")).toBe(1);
  });

  it("counts Core's 7-day trailing recent window as 7", () => {
    expect(daysInclusive("2026-01-09", "2026-01-15")).toBe(7);
  });
});
