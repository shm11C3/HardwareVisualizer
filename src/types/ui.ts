export type SelectedDisplayType =
  | "dashboard"
  | "groupedDashboard"
  | "performance"
  | "usage"
  | "cpuDetail"
  | "insights"
  | "settings";

export const insightChildMenu = ["main", "gpu"] as const;

export type InsightChildMenuType = (typeof insightChildMenu)[number];
