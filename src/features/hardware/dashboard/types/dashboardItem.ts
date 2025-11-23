export const dashBoardItemType = [
  "cpu",
  "gpu",
  "memory",
  "storage",
  "process",
  "network",
] as const;

export type DashboardItemType = (typeof dashBoardItemType)[number];

export type DashboardSelectItemType = DashboardItemType | "title";
