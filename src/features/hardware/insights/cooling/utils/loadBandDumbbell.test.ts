import { describe, expect, it } from "vitest";
import type { CoolingBandComparisonEntry } from "@/rspc/bindings";
import {
  buildAmbientAdjustedDumbbellRows,
  buildLoadBandDumbbellRows,
  positionPercent,
} from "./loadBandDumbbell";

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

describe("buildAmbientAdjustedDumbbellRows", () => {
  it("returns null when no band carries an ambient reading", () => {
    // The normal state on a machine with no environmental sensor: the
    // panel must then render exactly as it did before #2046, so the
    // absence has to be distinguishable from "present but not comparable".
    expect(
      buildAmbientAdjustedDumbbellRows(
        [entry({ band: "idle" }), entry({ band: "low" })],
        "C",
      ),
    ).toBeNull();
  });

  it("reads the thermal delta rather than the absolute temperature", () => {
    const rows = buildAmbientAdjustedDumbbellRows(
      [
        entry({
          ambientAdjusted: {
            baseline: { deltaAvg: 28, sampleMinutes: 11_000 },
            recent: { deltaAvg: 28, sampleMinutes: 5_400 },
            comparable: true,
          },
        }),
      ],
      "C",
    );

    // The absolute reading on this entry rose 1.5 degC; above ambient it
    // did not move at all, which is the whole point of the variant.
    expect(rows).toEqual([
      { band: "idle", comparable: true, baseline: 28, recent: 28, delta: 0 },
    ]);
  });

  it("converts both endpoints as spans, since a ΔT is a difference", () => {
    const rows = buildAmbientAdjustedDumbbellRows(
      [
        entry({
          ambientAdjusted: {
            baseline: { deltaAvg: 28, sampleMinutes: 11_000 },
            recent: { deltaAvg: 33, sampleMinutes: 5_400 },
            comparable: true,
          },
        }),
      ],
      "F",
    );
    const row = rows?.[0];
    if (row == null || !row.comparable) {
      throw new Error("expected a comparable row");
    }
    // No +32 offset anywhere: 28 K above ambient is 50.4 R above ambient,
    // not 82.4.
    expect(row.baseline).toBeCloseTo(28 * 1.8);
    expect(row.recent).toBeCloseTo(33 * 1.8);
    expect(row.delta).toBeCloseTo(5 * 1.8);
  });

  it("keeps a band whose window is too thin honestly not comparable", () => {
    const rows = buildAmbientAdjustedDumbbellRows(
      [
        entry({
          ambientAdjusted: {
            baseline: { deltaAvg: 28, sampleMinutes: 11_000 },
            recent: { deltaAvg: null, sampleMinutes: 8 },
            comparable: false,
          },
        }),
      ],
      "C",
    );

    expect(rows).toEqual([{ band: "idle", comparable: false }]);
  });

  it("renders a band with no ambient pairing beside bands that have one", () => {
    // A band can be absent from the ambient reading while its neighbours
    // are not; dropping the row would silently renumber the chart.
    const rows = buildAmbientAdjustedDumbbellRows(
      [
        entry({
          band: "idle",
          ambientAdjusted: {
            baseline: { deltaAvg: 28, sampleMinutes: 11_000 },
            recent: { deltaAvg: 29, sampleMinutes: 5_400 },
            comparable: true,
          },
        }),
        entry({ band: "high", ambientAdjusted: null }),
      ],
      "C",
    );

    expect(rows?.map((row) => [row.band, row.comparable])).toEqual([
      ["idle", true],
      ["high", false],
    ]);
  });

  it("falls back to not-comparable if a delta is missing despite the flag", () => {
    const rows = buildAmbientAdjustedDumbbellRows(
      [
        entry({
          ambientAdjusted: {
            baseline: { deltaAvg: null, sampleMinutes: 0 },
            recent: { deltaAvg: 29, sampleMinutes: 5_400 },
            comparable: true,
          },
        }),
      ],
      "C",
    );

    expect(rows).toEqual([{ band: "idle", comparable: false }]);
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
