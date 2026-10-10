import { atom } from "jotai";
import type { SelectedDisplayType } from "@/types/ui";

export const showTitleIconAtom = atom<SelectedDisplayType[]>([
  "dashboard",
  "cpuDetail",
  "insights",
  "settings",
]);
