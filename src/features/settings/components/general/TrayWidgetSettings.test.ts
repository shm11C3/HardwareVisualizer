import { describe, expect, it } from "vitest";
import {
  normalizeMetricOrder,
  normalizeSettings,
  normalizeVisibleMetrics,
  type TrayWidgetStore,
} from "./TrayWidgetSettings";

describe("TrayWidgetSettings normalization", () => {
  it("returns null for pending settings", () => {
    expect(normalizeSettings(null)).toBeNull();
  });

  it("normalizes legacy metrics into order and visibility", () => {
    const settings = normalizeSettings({
      metrics: ["gpu", "temp", "cpu"],
      updateIntervalSecs: 2,
    });

    expect(settings).toEqual({
      enabled: false,
      metricOrder: ["gpu", "cpu"],
      visibleMetrics: ["gpu", "cpu"],
      updateIntervalSecs: 2,
    });
  });

  it("keeps metric order and filters visible metrics by that order", () => {
    const settings = normalizeSettings({
      enabled: true,
      metricOrder: ["gpu", "cpu"],
      visibleMetrics: ["cpu", "temp"],
      updateIntervalSecs: 1,
    });

    expect(settings).toEqual({
      enabled: true,
      metricOrder: ["gpu", "cpu"],
      visibleMetrics: ["cpu"],
      updateIntervalSecs: 1,
    });
  });

  it("removes duplicate and non-configurable metrics from metric order", () => {
    expect(normalizeMetricOrder(["gpu", "temp", "gpu", "cpu"])).toEqual([
      "gpu",
      "cpu",
    ]);
  });

  it("removes duplicate and non-configurable visible metrics", () => {
    expect(
      normalizeVisibleMetrics(["temp", "gpu", "gpu", "cpu"], ["cpu", "gpu"]),
    ).toEqual(["gpu", "cpu"]);
  });

  it("falls back to the default interval when persisted interval is invalid", () => {
    const settings = normalizeSettings({
      enabled: true,
      metricOrder: ["gpu", "cpu"],
      visibleMetrics: ["gpu"],
      updateIntervalSecs: 99,
    });

    expect(settings?.updateIntervalSecs).toBe(1);
  });

  it("falls back to metric order when visible metrics normalize to empty", () => {
    expect(normalizeVisibleMetrics(["temp"], ["gpu", "cpu"])).toEqual([
      "gpu",
      "cpu",
    ]);
  });

  it("handles completely empty persisted objects", () => {
    const settings = normalizeSettings({} as Partial<TrayWidgetStore>);

    expect(settings).toEqual({
      enabled: false,
      metricOrder: ["cpu", "gpu"],
      visibleMetrics: ["cpu", "gpu"],
      updateIntervalSecs: 1,
    });
  });
});
