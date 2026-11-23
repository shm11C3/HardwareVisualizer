export const chartHardwareTypes = ["cpu", "memory", "gpu"] as const;

export type ChartDataType = (typeof chartHardwareTypes)[number];

export type HardwareDataType = "temp" | "usage" | "clock" | "memoryUsageValue";

export type GpuDataType = "temp" | "usage" | "dedicatedMemory";

export const isChartDataType = (param: unknown): param is ChartDataType => {
  return (
    typeof param === "string" &&
    ([...chartHardwareTypes] as string[]).includes(param)
  );
};
