import { describe, expect, it } from "vitest";
import { archivePeriods } from "@/features/hardware/consts/chart";
import { describeArchivePeriod } from "./archivePeriodLabel";

describe("describeArchivePeriod", () => {
  it("keeps sub-hour periods in minutes", () => {
    expect(describeArchivePeriod(10)).toEqual({ count: 10, unit: "minutes" });
    expect(describeArchivePeriod(30)).toEqual({ count: 30, unit: "minutes" });
  });

  it("describes whole hours below a day in hours", () => {
    expect(describeArchivePeriod(60)).toEqual({ count: 1, unit: "hours" });
    expect(describeArchivePeriod(180)).toEqual({ count: 3, unit: "hours" });
    expect(describeArchivePeriod(720)).toEqual({ count: 12, unit: "hours" });
  });

  it("describes whole days in days", () => {
    expect(describeArchivePeriod(1440)).toEqual({ count: 1, unit: "days" });
    expect(describeArchivePeriod(10080)).toEqual({ count: 7, unit: "days" });
    expect(describeArchivePeriod(20160)).toEqual({ count: 14, unit: "days" });
    expect(describeArchivePeriod(43200)).toEqual({ count: 30, unit: "days" });
  });

  it("describes every selectable period with a whole count", () => {
    for (const period of archivePeriods) {
      const { count } = describeArchivePeriod(period);
      expect(Number.isInteger(count)).toBe(true);
      expect(count).toBeGreaterThan(0);
    }
  });
});
