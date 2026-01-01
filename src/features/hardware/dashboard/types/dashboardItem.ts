export const dashBoardItems = [
  "cpu",
  "gpu",
  "memory",
  "storage",
  "process",
  "network",
] as const;

export type DashboardItemType = (typeof dashBoardItems)[number];

export type DashboardSelectItemType = DashboardItemType | "title";
