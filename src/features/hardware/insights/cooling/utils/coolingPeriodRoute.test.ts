import { describe, expect, it } from "vitest";
import { resolveCoolingPeriodRoute } from "./coolingPeriodRoute";

describe("resolveCoolingPeriodRoute", () => {
  it("routes 24h to the 1-day archive bucket query", () => {
    expect(resolveCoolingPeriodRoute("24h")).toEqual({
      kind: "archive",
      minutes: 1440,
    });
  });

  it("routes 7d to the 7-day archive bucket query", () => {
    expect(resolveCoolingPeriodRoute("7d")).toEqual({
      kind: "archive",
      minutes: 10080,
    });
  });

  it("routes 30d to the 30-day archive bucket query", () => {
    expect(resolveCoolingPeriodRoute("30d")).toEqual({
      kind: "archive",
      minutes: 43200,
    });
  });

  it("routes 90d to the daily rollup trend query", () => {
    expect(resolveCoolingPeriodRoute("90d")).toEqual({
      kind: "dailyTrend",
      days: 90,
    });
  });

  it("routes 1y to the 365-day daily rollup trend query", () => {
    expect(resolveCoolingPeriodRoute("1y")).toEqual({
      kind: "dailyTrend",
      days: 365,
    });
  });
});
