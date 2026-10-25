import type { ChartDataType } from "@/types/chart";

export const displayHardType: Record<ChartDataType, string> = {
  cpu: "CPU",
  memory: "RAM",
  gpu: "GPU",
} as const;

/**
 * Display period for insight feature, in minutes
 */
export const archivePeriods = [
  10, 30, 60, 180, 720, 1440, 10080, 20160, 43200, 129600, 525600,
] as const;

/**
 * The periods Process Insight offers. It stops at 30 days because it renders
 * every process in the period with no top-N or paging yet; longer periods
 * wait for those display limits (#672).
 */
export const processStatsPeriods = [
  10, 30, 60, 180, 720, 1440, 10080, 20160, 43200,
] as const satisfies readonly (typeof archivePeriods)[number][];

export const bubbleChartColor = "#8884d8";
