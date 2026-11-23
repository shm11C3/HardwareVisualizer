import { describe, expect, it } from "vitest";
import { formatLinkSpeed } from "./formatLinkSpeed";

describe("formatLinkSpeed", () => {
  it.each([
    [2_500_000_000, "2.5", "gigabitsPerSecond"],
    [1_000_000_000, "1", "gigabitsPerSecond"],
    [1_201_000_000, "1.2", "gigabitsPerSecond"],
    [866_700_000, "867", "megabitsPerSecond"],
    [100_000_000, "100", "megabitsPerSecond"],
    [54_000_000, "54", "megabitsPerSecond"],
    [400_000, "0.4", "megabitsPerSecond"],
  ])(
    "shows %i bit/s as %s in decimal units without a trailing zero",
    (bps, value, unit) => {
      expect(formatLinkSpeed(bps, "en")).toEqual({ value, unit });
    },
  );

  it("rolls a Mb/s value that rounds to 1000 over to Gb/s", () => {
    // protects against the odd label "1,000 Mb/s" next to "1 Gb/s" elsewhere
    expect(formatLinkSpeed(999_600_000, "en")).toEqual({
      value: "1",
      unit: "gigabitsPerSecond",
    });
  });

  it("formats the number with the given locale's decimal separator", () => {
    expect(formatLinkSpeed(2_500_000_000, "ru").value).toBe("2,5");
  });
});
