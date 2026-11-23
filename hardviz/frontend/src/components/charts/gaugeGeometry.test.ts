import { describe, expect, it } from "vitest";
import {
  gaugeFraction,
  gaugeRingDash,
} from "@/components/charts/gaugeGeometry";

describe("gaugeFraction", () => {
  it("maps a percentage straight onto the ring", () => {
    expect(
      gaugeFraction({
        chartValue: 65,
        dataType: "usage",
        temperatureUnit: "C",
      }),
    ).toBeCloseTo(0.65, 5);
  });

  it("plots the same temperature identically in either unit", () => {
    const celsius = gaugeFraction({
      chartValue: 50,
      dataType: "temp",
      temperatureUnit: "C",
    });
    const fahrenheit = gaugeFraction({
      chartValue: 122, // 50°C
      dataType: "temp",
      temperatureUnit: "F",
    });

    expect(fahrenheit).toBeCloseTo(celsius, 5);
  });

  it("uses the usage percentage for a memory reading, not its value", () => {
    expect(
      gaugeFraction({
        chartValue: 19,
        dataType: "memoryUsageValue",
        usagePercentage: 67,
        temperatureUnit: "C",
      }),
    ).toBeCloseTo(0.67, 5);
  });

  it("pins the ring at the ends of the scale instead of wrapping", () => {
    expect(
      gaugeFraction({
        chartValue: 140,
        dataType: "usage",
        temperatureUnit: "C",
      }),
    ).toBe(1);
    expect(
      gaugeFraction({
        chartValue: -10,
        dataType: "temp",
        temperatureUnit: "C",
      }),
    ).toBe(0);
  });
});

describe("gaugeRingDash", () => {
  it("hides the whole ring at zero and reveals all of it at one", () => {
    const radius = 55;
    const circumference = 2 * Math.PI * radius;

    expect(gaugeRingDash(0, radius)).toEqual({
      strokeDasharray: circumference,
      strokeDashoffset: circumference,
    });
    expect(gaugeRingDash(1, radius)).toEqual({
      strokeDasharray: circumference,
      strokeDashoffset: 0,
    });
  });

  it("keeps the dash pattern fixed so only the offset moves per tick", () => {
    const quarter = gaugeRingDash(0.25, 55);
    const half = gaugeRingDash(0.5, 55);

    expect(half.strokeDasharray).toBe(quarter.strokeDasharray);
    expect(half.strokeDashoffset).toBeCloseTo(
      quarter.strokeDashoffset - quarter.strokeDasharray * 0.25,
      5,
    );
  });
});
