import { describe, expect, it } from "vitest";
import type { CoolingBandComparisonEntry } from "@/rspc/bindings";
import { buildLoadBandDumbbellRows, positionPercent } from "./loadBandDumbbell";

const entry = (
  overrides: Partial<CoolingBandComparisonEntry> = {},
): CoolingBandComparisonEntry => ({
  band: "idle",
  baseline: { temperatureAvg: 32, sampleMinutes: 12_600 },
  recent: { temperatureAvg: 33.5, sampleMinutes: 6_300 },
  comparable: true,
  // These rows read absolute temperature only; the ambient-adjusted
  // reading (#2045) is rendered separately by #2046.
  ambientAdjusted: null,
  ...overrides,
});

describe("buildLoadBandDumbbellRows", () => {
  it("converts a comparable band's baseline/recent/delta in Celsius", () => {
    const [row] = buildLoadBandDumbbellRows([entry()], "C");

    expect(row).toEqual({
      band: "idle",
      comparable: true,
      baseline: 32,
      recent: 33.5,
      delta: 1.5,
    });
  });

  it("scales the delta by 9/5 with no +32 offset in Fahrenheit", () => {
    const [row] = buildLoadBandDumbbellRows([entry()], "F");

    if (!row.comparable) {
      throw new Error("expected a comparable row");
    }
    // Absolute points do get the +32 offset; the delta must not.
    expect(row.baseline).toBeCloseTo(32 * 1.8 + 32);
    expect(row.recent).toBeCloseTo(33.5 * 1.8 + 32);
    expect(row.delta).toBeCloseTo(1.5 * 1.8);
  });

  it("marks a band Core reported as not comparable", () => {
    const [row] = buildLoadBandDumbbellRows(
      [
        entry({
          comparable: false,
          recent: { temperatureAvg: null, sampleMinutes: 40 },
        }),
      ],
      "C",
    );

    expect(row).toEqual({ band: "idle", comparable: false });
  });

  it("falls back to not-comparable if a temperature is missing despite the flag", () => {
    const [row] = buildLoadBandDumbbellRows(
      [
        entry({
          comparable: true,
          baseline: { temperatureAvg: null, sampleMinutes: 0 },
        }),
      ],
      "C",
    );

    expect(row).toEqual({ band: "idle", comparable: false });
  });

  it("preserves band order across multiple entries", () => {
    const rows = buildLoadBandDumbbellRows(
      [
        entry({ band: "idle" }),
        entry({ band: "low" }),
        entry({ band: "mid", comparable: false }),
      ],
      "C",
    );

    expect(rows.map((row) => row.band)).toEqual(["idle", "low", "mid"]);
  });
});

describe("positionPercent", () => {
  it("maps the domain min/max to 0/100", () => {
    expect(positionPercent(30, [30, 40])).toBe(0);
    expect(positionPercent(40, [30, 40])).toBe(100);
  });

  it("maps the domain midpoint to 50", () => {
    expect(positionPercent(35, [30, 40])).toBe(50);
  });

  it("clamps values outside the domain", () => {
    expect(positionPercent(20, [30, 40])).toBe(0);
    expect(positionPercent(50, [30, 40])).toBe(100);
  });

  it("centers a degenerate domain instead of dividing by zero", () => {
    expect(positionPercent(30, [30, 30])).toBe(50);
  });
});
