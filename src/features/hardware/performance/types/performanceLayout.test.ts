import { describe, expect, it } from "vitest";
import {
  DEFAULT_PERFORMANCE_CUSTOM_LAYOUT,
  DEFAULT_PERFORMANCE_POWER_MODE,
  DEFAULT_PERFORMANCE_VIEW,
  normalizePerformanceCustomLayout,
  normalizePerformancePowerMode,
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

describe("Performance Power Draw mode normalization", () => {
  it("keeps supported modes", () => {
    expect(normalizePerformancePowerMode("current")).toBe("current");
    expect(normalizePerformancePowerMode("graph")).toBe("graph");
  });

  it("falls back to Current for an unknown mode", () => {
    expect(normalizePerformancePowerMode("overlay")).toBe(
      DEFAULT_PERFORMANCE_POWER_MODE,
    );
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
      visible: ["processTable"],
    });
  });

  it("appends new panels without overriding stored visibility", () => {
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
      visible: ["usageGraphs", "processTable"],
    });
  });

  it("preserves a limited stored visibility selection", () => {
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
      visible: ["usageGraphs"],
    });
  });

  it("preserves an explicit empty stored visibility selection", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["usageGraphs", "processTable", "perCore", "motherboardSensors"],
        visible: [],
      }).visible,
    ).toEqual([]);
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
      visible: ["usageGraphs"],
    });
  });

  it("uses default visibility when the stored field is malformed", () => {
    expect(
      normalizePerformanceCustomLayout({
        order: ["usageGraphs"],
        visible: "usageGraphs",
      }).visible,
    ).toEqual(DEFAULT_PERFORMANCE_CUSTOM_LAYOUT.visible);
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
