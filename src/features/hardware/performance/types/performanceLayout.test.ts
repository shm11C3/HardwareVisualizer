import { describe, expect, it } from "vitest";
import {
  DEFAULT_PERFORMANCE_PRESET,
  normalizePerformanceCustomLayout,
  normalizePerformancePreset,
  performanceCustomLayoutsEqual,
} from "./performanceLayout";

describe("performance layout normalization", () => {
  it("falls back to Detailed for an unknown preset", () => {
    expect(normalizePerformancePreset("analysis")).toBe(
      DEFAULT_PERFORMANCE_PRESET,
    );
  });

  it("keeps known order and visibility while dropping unknown panels", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["processTable", "futurePanel", "currentValues"],
        visible: ["processTable", "futurePanel"],
      }),
    ).toEqual({
      order: ["processTable", "currentValues", "usageGraphs"],
      visible: ["processTable", "usageGraphs"],
    });
  });

  it("appends newly introduced panels without restoring panels the user hid", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["usageGraphs", "currentValues"],
        visible: ["currentValues"],
      }),
    ).toEqual({
      order: ["usageGraphs", "currentValues", "processTable"],
      visible: ["currentValues", "processTable"],
    });
  });

  it("repairs an empty visible selection", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["processTable", "currentValues", "usageGraphs"],
        visible: [],
      }).visible,
    ).toEqual(["currentValues"]);
  });

  it("treats malformed persisted objects as unequal so they are rewritten", () => {
    expect(
      performanceCustomLayoutsEqual({}, normalizePerformanceCustomLayout({})),
    ).toBe(false);
  });
});
