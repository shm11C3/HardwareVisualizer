import { describe, expect, it } from "vitest";
import {
  DEFAULT_PERFORMANCE_CUSTOM_LAYOUT,
  DEFAULT_PERFORMANCE_VIEW,
  normalizePerformanceCustomLayout,
  normalizePerformanceView,
  performanceCustomLayoutsEqual,
} from "./performanceLayout";

describe("performance view normalization", () => {
  it("falls back to Panels for an unknown view", () => {
    expect(normalizePerformanceView("analysis")).toBe(DEFAULT_PERFORMANCE_VIEW);
  });

  it("maps the retired Detailed and Custom presets onto Panels", () => {
    expect(normalizePerformanceView("detailed")).toBe("panels");
    expect(normalizePerformanceView("custom")).toBe("panels");
  });

  it("keeps Compact and Monitor selections", () => {
    expect(normalizePerformanceView("compact")).toBe("compact");
    expect(normalizePerformanceView("monitor")).toBe("monitor");
  });
});

describe("performance layout normalization", () => {
  it("keeps known order and visibility while dropping unknown panels", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["processTable", "futurePanel", "usageGraphs"],
        visible: ["processTable", "futurePanel"],
      }),
    ).toEqual({
      order: [
        "processTable",
        "usageGraphs",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["processTable", "power"],
    });
  });

  it("appends new panels according to their default visibility", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["usageGraphs", "processTable"],
        visible: ["usageGraphs", "processTable"],
      }),
    ).toEqual({
      order: [
        "usageGraphs",
        "processTable",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["usageGraphs", "processTable", "power"],
    });
  });

  it("makes a missing default-visible panel visible again", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["usageGraphs"],
        visible: ["usageGraphs"],
      }),
    ).toEqual({
      order: [
        "usageGraphs",
        "processTable",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["usageGraphs", "processTable", "power"],
    });
  });

  it("adds a newly introduced default-visible panel to an older empty layout", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
        visible: [],
      }).visible,
    ).toEqual(["power"]);
  });

  it("drops the retired currentValues panel from legacy layouts", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["currentValues", "usageGraphs", "processTable"],
        visible: ["currentValues", "usageGraphs"],
      }),
    ).toEqual({
      order: [
        "usageGraphs",
        "processTable",
        "perCore",
        "motherboardSensors",
        "power",
      ],
      visible: ["usageGraphs", "power"],
    });
  });

  it("falls back to the default layout for malformed values", () => {
    expect(normalizePerformanceCustomLayout(null)).toEqual(
      DEFAULT_PERFORMANCE_CUSTOM_LAYOUT,
    );
  });

  it("treats malformed persisted objects as unequal so they are rewritten", () => {
    expect(
      performanceCustomLayoutsEqual({}, normalizePerformanceCustomLayout({})),
    ).toBe(false);
  });
});
