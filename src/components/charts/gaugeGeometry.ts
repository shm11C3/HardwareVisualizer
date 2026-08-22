import type { HardwareDataType } from "@/features/hardware/types/hardwareDataType";
import type { Settings } from "@/features/settings/types/settingsType";

/**
 * The gauge sweeps a full turn at 100. Temperatures are plotted on the same
 * 0-100 scale, so a Fahrenheit reading is converted back to Celsius first —
 * otherwise the same temperature would fill a different amount of the ring
 * depending on the unit the user picked.
 */
export const gaugeFraction = ({
  chartValue,
  dataType,
  usagePercentage,
  temperatureUnit,
}: {
  chartValue: number;
  dataType: HardwareDataType;
  usagePercentage?: number | undefined;
  temperatureUnit: Settings["temperatureUnit"];
}): number => {
  const scaled = (() => {
    if (dataType === "memoryUsageValue") {
      return usagePercentage ?? 0;
    }

    return dataType === "temp" && temperatureUnit === "F"
      ? (chartValue - 32) / 1.8
      : chartValue;
  })();

  // A reading past the ends of the scale pins the ring rather than wrapping
  // past the start, which would read as a much lower value.
  return Math.min(Math.max(scaled / 100, 0), 1);
};

/**
 * Dash geometry for a ring drawn as a stroked circle.
 *
 * The arc is the stroke's visible run, so a tick only has to move
 * `strokeDashoffset` — a property the compositor can transition on its own,
 * without React rendering a frame.
 */
export const gaugeRingDash = (
  fraction: number,
  radius: number,
): { strokeDasharray: number; strokeDashoffset: number } => {
  const circumference = 2 * Math.PI * radius;

  return {
    strokeDasharray: circumference,
    strokeDashoffset: circumference * (1 - fraction),
  };
};
