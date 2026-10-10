import { atom } from "jotai";
import type { SelectedDisplayType } from "@/types/ui";

export const displayTargetAtom = atom<SelectedDisplayType | null>(null);
export const sideMenuOpenAtom = atom<boolean | null>(null);
export const navigationLayoutFocusRequestedAtom = atom(false);
