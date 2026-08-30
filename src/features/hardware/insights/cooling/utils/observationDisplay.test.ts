import { describe, expect, it } from "vitest";
import { daysInclusive, resolveObservationDisplay } from "./observationDisplay";

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

describe("daysInclusive", () => {
  it("counts a single day as 1", () => {
    expect(daysInclusive("2026-01-15", "2026-01-15")).toBe(1);
  });

  it("counts Core's 7-day trailing recent window as 7", () => {
    expect(daysInclusive("2026-01-09", "2026-01-15")).toBe(7);
  });
});
