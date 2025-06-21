export const dashBoardItemType = [
  "cpu",
  "gpu",
  "memory",
  "storage",
  "process",
  "storage",
  "network",
] as const;

export type DashboardItemType = (typeof dashBoardItemType)[number];
