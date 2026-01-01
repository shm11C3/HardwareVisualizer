import type { ChartDataType } from "@/features/hardware/types/hardwareDataType";

export const chartConfig = {
  /**
   * Length of graph history (in seconds)
   */
  historyLengthSec: 60,
  archiveUpdateIntervalMilSec: 60000,
} as const;

export const displayHardType: Record<ChartDataType, string> = {
  cpu: "CPU",
  memory: "RAM",
  gpu: "GPU",
} as const;

export const sizeOptions = ["sm", "md", "lg", "xl", "2xl"] as const;

export const defaultColorRGB: Record<ChartDataType, string> = {
  cpu: "75, 192, 192",
  memory: "255, 99, 132",
  gpu: "255, 206, 86",
};

/**
 * Display period for insight feature
 */
export const archivePeriods = [
  10, 30, 60, 180, 720, 1440, 10080, 20160, 43200,
] as const;

export const bubbleChartColor = "#8884d8";
