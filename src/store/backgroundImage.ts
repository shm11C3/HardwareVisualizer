import { atom } from "jotai";
import type { BackgroundImage } from "@/rspc/bindings";

export const backgroundImageAtom = atom<string | null>(null);
export const uploadedBackgroundImagesAtom = atom<Array<BackgroundImage>>([]);
