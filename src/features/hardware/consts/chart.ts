import type { ChartDataType } from "@/types/chart";

export const displayHardType: Record<ChartDataType, string> = {
  cpu: "CPU",
  memory: "RAM",
  gpu: "GPU",
} as const;

/**
 * Display period for insight feature
 */
export const archivePeriods = [
  10, 30, 60, 180, 720, 1440, 10080, 20160, 43200,
] as const;

export const bubbleChartColor = "#8884d8";
