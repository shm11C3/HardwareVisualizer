import { atom } from "jotai";
import type { BackgroundImage } from "@/rspc/bindings";

export const backgroundImageAtom = atom<string | null>(null);
export const uploadedBackgroundImagesAtom = atom<Array<BackgroundImage>>([]);

/**
 * Which background image action the user performed last failed (user-action
 * tier of the failure-reporting policy). The hooks are mounted from several
 * places, so the state is shared here and each control renders the action it
 * owns: `upload` under the upload control, `delete` under the image list.
 * Selecting an image is an Application Preference write and is attributed by
 * its setting key (`selectedBackgroundImg`) instead. Only the next success of
 * the same action clears it, never an unrelated one. Automatic reads (the
 * list and the selected image at startup) log only. The backend detail goes
 * to the log, never to the UI.
 */
export type BackgroundImageFailure = "upload" | "delete";

export const backgroundImageFailureAtom = atom<BackgroundImageFailure | null>(
  null,
);
