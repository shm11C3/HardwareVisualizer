export const chartHardwareTypes = ["cpu", "memory", "gpu"] as const;

export type ChartDataType = (typeof chartHardwareTypes)[number];

export type HardwareDataType = "temp" | "usage" | "clock";

export type GpuDataType = "temp" | "usage" | "dedicatedMemory";

export type NameValues = Array<{
  name: string;
  value: number;
}>;

export type DataStats = "avg" | "max" | "min";
