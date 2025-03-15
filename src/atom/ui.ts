import type { InsightChildMenuType } from "@/types/ui";
import { atom } from "jotai";

export const modalAtoms = {
  showSettingsModal: atom<boolean>(false),
};

export const insightMenuAtom = atom<InsightChildMenuType>("main");
