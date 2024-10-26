import type { ChartDataType, HardwareDataType } from "@/types/hardwareDataType";

export const chartConfig = {
  /**
   * グラフの履歴の長さ（秒）
   */
  historyLengthSec: 60,
} as const;

export const displayHardType: Record<ChartDataType, string> = {
  cpu: "CPU",
  memory: "RAM",
  gpu: "GPU",
} as const;

export const displayDataType: Record<HardwareDataType, string> = {
  temp: "Temperature",
  usage: "Usage",
  clock: "Clock",
} as const;

export const sizeOptions = ["sm", "md", "lg", "xl", "2xl"] as const;

export const defaultColorRGB: Record<ChartDataType, string> = {
  cpu: "75, 192, 192",
  memory: "255, 99, 132",
  gpu: "255, 206, 86",
};
