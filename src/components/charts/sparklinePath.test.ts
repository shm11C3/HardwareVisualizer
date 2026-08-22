import { describe, expect, it } from "vitest";
import {
  buildSparklinePath,
  sparklineTicks,
  sparklineViewBox,
} from "@/components/charts/sparklinePath";

const range: [number, number] = [0, 100];

describe("buildSparklinePath", () => {
  it("maps the value range onto the full height, top-down", () => {
    const { line } = buildSparklinePath({
      values: [0, 50, 100],
      range,
      lineGraphType: "linear",
    });

    // 0% sits on the baseline, 100% on the top edge.
    expect(line).toBe("M0,100L50,50L100,0");
  });

  it("spreads points evenly across the full width", () => {
    const { line } = buildSparklinePath({
      values: [10, 10, 10, 10, 10],
      range,
      lineGraphType: "linear",
    });

    expect(line.startsWith("M0,90")).toBe(true);
    expect(line.endsWith(`L${sparklineViewBox.width},90`)).toBe(true);
  });

  it("breaks the line at missing readings instead of drawing them as zero", () => {
    const { line } = buildSparklinePath({
      values: [50, null, 50],
      range,
      lineGraphType: "linear",
    });

    // Two separate subpaths, and no point on the baseline for the gap.
    expect(line.match(/M/g)).toHaveLength(2);
    expect(line).not.toContain(",100");
  });

  it("skips the leading nulls of a history buffer that is not full yet", () => {
    const { line } = buildSparklinePath({
      values: [null, null, 25, 75],
      range,
      lineGraphType: "linear",
    });

    expect(line.match(/M/g)).toHaveLength(1);
    expect(line.startsWith("M66.66")).toBe(true);
  });

  it("closes the area down to the baseline", () => {
    const { area } = buildSparklinePath({
      values: [50, 50],
      range,
      lineGraphType: "linear",
    });

    expect(area).toContain(`L${sparklineViewBox.width},100`);
    expect(area.endsWith("Z")).toBe(true);
  });

  it("honours the configured range", () => {
    const { line } = buildSparklinePath({
      values: [2000, 4000],
      range: [0, 4000],
      lineGraphType: "linear",
    });

    expect(line).toBe("M0,50L100,0");
  });

  it("emits curve commands for the non-linear graph types", () => {
    const values = [10, 80, 30, 90];

    expect(
      buildSparklinePath({ values, range, lineGraphType: "default" }).line,
    ).toContain("C");
    expect(
      buildSparklinePath({ values, range, lineGraphType: "basis" }).line,
    ).toContain("C");
    // Recharts' "step" resolves to d3's curveStep, which is orthogonal moves.
    expect(
      buildSparklinePath({ values, range, lineGraphType: "step" }).line,
    ).toMatch(/^M[\d.,]+(L[\d.,]+)+$/);
  });

  it("returns empty paths when every reading is missing", () => {
    const { line, area } = buildSparklinePath({
      values: [null, null],
      range,
      lineGraphType: "linear",
    });

    expect(line).toBe("");
    expect(area).toBe("");
  });
});

describe("sparklineTicks", () => {
  it("labels the range highest-first, spanning both edges", () => {
    expect(sparklineTicks([0, 100], 3)).toEqual([
      { value: 100, y: 0 },
      { value: 50, y: 50 },
      { value: 0, y: 100 },
    ]);
  });

  it("labels every 20 across a 0-100 range at the scale tick count", () => {
    const ticks = sparklineTicks([0, 100], 6);

    expect(ticks).toHaveLength(6);
    expect(ticks.map((tick) => tick.value)).toEqual([100, 80, 60, 40, 20, 0]);
    expect(ticks.map((tick) => tick.y)).toEqual([0, 20, 40, 60, 80, 100]);
  });

  it("labels a non-default range", () => {
    expect(sparklineTicks([0, 4000], 5).map((tick) => tick.value)).toEqual([
      4000, 3000, 2000, 1000, 0,
    ]);
  });

  it("returns nothing when a scale cannot be drawn", () => {
    expect(sparklineTicks([0, 100], 1)).toEqual([]);
    expect(sparklineTicks([0, 100], 0)).toEqual([]);
  });
});
