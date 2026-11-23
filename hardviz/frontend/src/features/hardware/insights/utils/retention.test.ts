import { describe, expect, it } from "vitest";
import { periodExceedsRetention } from "./retention";

describe("periodExceedsRetention", () => {
  it("is true when the period reaches past the Retention Period", () => {
    expect(
      periodExceedsRetention(525600, {
        scheduledDataDeletion: true,
        retentionDays: 30,
      }),
    ).toBe(true);
  });

  it("is false when the Retention Period covers the period", () => {
    expect(
      periodExceedsRetention(525600, {
        scheduledDataDeletion: true,
        retentionDays: 365,
      }),
    ).toBe(false);
    expect(
      periodExceedsRetention(43200, {
        scheduledDataDeletion: true,
        retentionDays: 30,
      }),
    ).toBe(false);
  });

  it("is false when scheduled deletion is off, because nothing is deleted", () => {
    expect(
      periodExceedsRetention(525600, {
        scheduledDataDeletion: false,
        retentionDays: 30,
      }),
    ).toBe(false);
  });

  it("is false when the Retention Period is unknown", () => {
    expect(
      periodExceedsRetention(525600, { scheduledDataDeletion: true }),
    ).toBe(false);
  });
});
