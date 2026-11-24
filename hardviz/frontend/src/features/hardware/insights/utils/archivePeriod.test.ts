import { describe, expect, it } from "vitest";
import {
  archivePeriods,
  processStatsPeriods,
} from "@/features/hardware/consts/chart";
import {
  insightBucketWidthMs,
  insightRefreshIntervalMs,
  normalizeArchivePeriod,
} from "./archivePeriod";

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;

describe("archive periods", () => {
  it("offers 90 days and 1 year after 30 days", () => {
    expect(archivePeriods.slice(-3)).toEqual([43200, 129600, 525600]);
  });

  it("accepts the long periods from stored values", () => {
    expect(normalizeArchivePeriod("129600")).toBe(129600);
    expect(normalizeArchivePeriod(525600)).toBe(525600);
  });

  it("keeps Process Insight at 30 days or shorter (#672)", () => {
    expect(Math.max(...processStatsPeriods)).toBe(43200);
    expect(processStatsPeriods).toEqual(archivePeriods.slice(0, -2));
  });
});

describe("insightBucketWidthMs", () => {
  it("keeps the existing widths for 30 days and shorter", () => {
    expect(insightBucketWidthMs(10)).toBe(MINUTE_MS);
    expect(insightBucketWidthMs(1440)).toBe(30 * MINUTE_MS);
    expect(insightBucketWidthMs(43200)).toBe(12 * HOUR_MS);
  });

  it("uses sub-day buckets for the long periods so no bucket claims to be a day", () => {
    expect(insightBucketWidthMs(129600)).toBe(6 * HOUR_MS);
    expect(insightBucketWidthMs(525600)).toBe(12 * HOUR_MS);
  });

  it("stays far under the Core point limit", () => {
    for (const period of archivePeriods) {
      const points = (period * MINUTE_MS) / insightBucketWidthMs(period);
      expect(points).toBeLessThanOrEqual(1_000);
    }
  });
});

describe("insightRefreshIntervalMs", () => {
  it("refreshes minute buckets every minute", () => {
    expect(insightRefreshIntervalMs(10)).toBe(MINUTE_MS);
  });

  it("follows the bucket width between one and five minutes", () => {
    expect(insightRefreshIntervalMs(720)).toBe(5 * MINUTE_MS);
    expect(insightRefreshIntervalMs(525600)).toBe(5 * MINUTE_MS);
  });
});
