export type SelectedDisplayType =
  | "dashboard"
  | "performance"
  | "hardwareCpu"
  | "hardwareGpu"
  | "hardwareMemory"
  | "hardwareStorage"
  | "hardwareSystem"
  | "usage"
  | "cpuDetail"
  | "insights"
  | "settings";

export const insightChildMenu = ["main", "gpu"] as const;

export type InsightChildMenuType = (typeof insightChildMenu)[number];
