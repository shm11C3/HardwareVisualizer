export type SelectedDisplayType =
  | "dashboard"
  | "usage"
  | "insights"
  | "settings";

export const insightChildMenu = ["main", "gpu"] as const;

export type InsightChildMenuType = (typeof insightChildMenu)[number];
