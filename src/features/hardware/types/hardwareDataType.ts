export const chartHardwareTypes = ["cpu", "memory", "gpu"] as const;

export type ChartDataType = (typeof chartHardwareTypes)[number];

export type HardwareDataType = "temp" | "usage" | "clock" | "memoryUsageValue";

export type GpuDataType = "temp" | "usage" | "dedicatedMemory";

export type NameValues = Array<{
  name: string;
  value: number;
}>;

export type FanSpeedStatus = "active" | "inactive" | "invalid";

export type MotherboardTemperatureValues = Array<{
  name: string;
  value: number;
  source: string;
}>;

export type MotherboardFanSpeedValues = Array<{
  name: string;
  rpm: number | null;
  status: FanSpeedStatus;
  source: string;
}>;

export type DataStats = "avg" | "max" | "min";

export const isChartDataType = (param: unknown): param is ChartDataType => {
  return (
    typeof param === "string" &&
    ([...chartHardwareTypes] as string[]).includes(param)
  );
};
