import { atom } from "jotai";
import type { DashboardItemType } from "../types/dashboardItem";

export const dashboardLayoutAtom = atom<DashboardItemType[]>([
  "cpu",
  "gpu",
  "memory",
  "storage",
  "network",
  "process",
]);
