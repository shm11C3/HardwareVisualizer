import { atom } from "jotai";
import type { Theme } from "@/rspc/bindings";

export const currentThemeAtom = atom<Exclude<Theme, "system"> | null>(null);
