import { describe, expect, it } from "vitest";
import {
  convertTemperatureDelta,
  formatSignedTemperatureDelta,
} from "./temperatureUnit";

describe("convertTemperatureDelta", () => {
  it("passes a Celsius delta through unchanged", () => {
    expect(convertTemperatureDelta(5, "C")).toBe(5);
    expect(convertTemperatureDelta(-3.5, "C")).toBe(-3.5);
  });

  it("scales a Fahrenheit delta by 9/5 with no +32 offset", () => {
    // A 5degC span is a 9degF span - not "41degF", which is what applying
    // the point-conversion offset to a delta would incorrectly produce.
    expect(convertTemperatureDelta(5, "F")).toBeCloseTo(9);
    expect(convertTemperatureDelta(-10, "F")).toBeCloseTo(-18);
    expect(convertTemperatureDelta(0, "F")).toBe(0);
  });
});

describe("formatSignedTemperatureDelta", () => {
  it("prefixes a positive delta with a plus sign", () => {
    expect(formatSignedTemperatureDelta(5.24, "°C")).toBe("+5.2°C");
  });

  it("prefixes a negative delta with a typographic minus sign", () => {
    expect(formatSignedTemperatureDelta(-4.9, "°C")).toBe("−4.9°C");
  });

  it("shows a delta that rounds to zero as +0.0 rather than -0.0", () => {
    expect(formatSignedTemperatureDelta(-0.04, "°C")).toBe("+0.0°C");
    expect(formatSignedTemperatureDelta(0, "°F")).toBe("+0.0°F");
  });
});
