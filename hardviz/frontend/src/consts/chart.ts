import type { ChartDataType } from "@/types/chart";

export const chartConfig = {
  /**
   * Length of graph history (in seconds)
   */
  historyLengthSec: 60,
  archiveUpdateIntervalMilSec: 60000,
} as const;

export const sizeOptions = ["sm", "md", "lg", "xl", "2xl"] as const;

export const defaultColorRGB: Record<ChartDataType, string> = {
  cpu: "75, 192, 192",
  memory: "255, 99, 132",
  gpu: "255, 206, 86",
};
