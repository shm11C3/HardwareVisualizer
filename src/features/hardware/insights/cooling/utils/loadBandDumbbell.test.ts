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
  comparability: "comparable",
  requiredSampleMinutes: 30,
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

  it("reports how many minutes a thin recent window still needs", () => {
    // Core's verdict and Core's minimum: the row says the recent side is
    // 18 minutes short and the baseline side is already sufficient.
    const [row] = buildLoadBandDumbbellRows(
      [
        entry({
          comparability: "tooFewSampleMinutes",
          recent: { temperatureAvg: 40, sampleMinutes: 12 },
        }),
      ],
      "C",
    );

    expect(row).toEqual({
      band: "idle",
      comparable: false,
      reason: {
        kind: "tooFewSampleMinutes",
        shortfall: {
          required: 30,
          baseline: { sampleMinutes: 12_600, remaining: 0 },
          recent: { sampleMinutes: 12, remaining: 18 },
        },
      },
    });
  });

  it("reports a baseline-side shortfall with the minutes it does have", () => {
    // The user's own case: the pinned baseline window held 12 high-load
    // minutes. That side never fills in, so the raw count matters.
    const [row] = buildLoadBandDumbbellRows(
      [
        entry({
          band: "high",
          comparability: "tooFewSampleMinutes",
          baseline: { temperatureAvg: 70, sampleMinutes: 12 },
          recent: { temperatureAvg: 72, sampleMinutes: 80 },
        }),
      ],
      "C",
    );

    expect(row).toMatchObject({
      comparable: false,
      reason: {
        kind: "tooFewSampleMinutes",
        shortfall: {
          baseline: { sampleMinutes: 12, remaining: 18 },
          recent: { sampleMinutes: 80, remaining: 0 },
        },
      },
    });
  });

  it("falls back to not-comparable if a temperature is missing despite the verdict", () => {
    const [row] = buildLoadBandDumbbellRows(
      [
        entry({
          comparability: "comparable",
          baseline: { temperatureAvg: null, sampleMinutes: 0 },
        }),
      ],
      "C",
    );

    expect(row).toEqual({
      band: "idle",
      comparable: false,
      reason: { kind: "missingValue" },
    });
  });

  it("preserves band order across multiple entries", () => {
    const rows = buildLoadBandDumbbellRows(
      [
        entry({ band: "idle" }),
        entry({ band: "low" }),
        entry({ band: "mid", comparability: "tooFewSampleMinutes" }),
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
            comparability: "comparable",
            requiredSampleMinutes: 30,
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
            comparability: "comparable",
            requiredSampleMinutes: 30,
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

  it("keeps a band whose window is too thin honestly not comparable, with its shortfall", () => {
    const rows = buildAmbientAdjustedDumbbellRows(
      [
        entry({
          ambientAdjusted: {
            baseline: { deltaAvg: 28, sampleMinutes: 11_000 },
            recent: { deltaAvg: null, sampleMinutes: 8 },
            comparability: "tooFewSampleMinutes",
            requiredSampleMinutes: 30,
          },
        }),
      ],
      "C",
    );

    expect(rows).toEqual([
      {
        band: "idle",
        comparable: false,
        reason: {
          kind: "tooFewSampleMinutes",
          shortfall: {
            required: 30,
            baseline: { sampleMinutes: 11_000, remaining: 0 },
            recent: { sampleMinutes: 8, remaining: 22 },
          },
        },
      },
    ]);
  });

  it("names the baseline's sensor when the recent window came from another one", () => {
    // Rich on both sides, so no amount of waiting changes the verdict:
    // the copy has to say which sensor the reference belongs to.
    const rows = buildAmbientAdjustedDumbbellRows(
      [
        entry({
          ambientAdjusted: {
            baseline: { deltaAvg: 26.7, sampleMinutes: 6_705 },
            recent: { deltaAvg: 25.1, sampleMinutes: 4_300 },
            comparability: "differentAmbientSource",
            requiredSampleMinutes: 30,
          },
        }),
      ],
      "C",
      "SwitchBot Meter (8a19)",
    );

    expect(rows).toEqual([
      {
        band: "idle",
        comparable: false,
        reason: {
          kind: "differentAmbientSource",
          baselineSource: "SwitchBot Meter (8a19)",
        },
      },
    ]);
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
            comparability: "comparable",
            requiredSampleMinutes: 30,
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
    expect(rows?.[1]).toMatchObject({ reason: { kind: "noAmbientPairing" } });
  });

  it("falls back to not-comparable if a delta is missing despite the verdict", () => {
    const rows = buildAmbientAdjustedDumbbellRows(
      [
        entry({
          ambientAdjusted: {
            baseline: { deltaAvg: null, sampleMinutes: 0 },
            recent: { deltaAvg: 29, sampleMinutes: 5_400 },
            comparability: "comparable",
            requiredSampleMinutes: 30,
          },
        }),
      ],
      "C",
    );

    expect(rows).toEqual([
      { band: "idle", comparable: false, reason: { kind: "missingValue" } },
    ]);
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
