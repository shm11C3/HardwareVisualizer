import { atom } from "jotai";
import type { BackgroundImage } from "@/rspc/bindings";

export const backgroundImageAtom = atom<string | null>(null);
export const uploadedBackgroundImagesAtom = atom<Array<BackgroundImage>>([]);

/**
 * User actions on the background image controls that can fail (user-action
 * tier of the failure-reporting policy): `upload` under the upload control,
 * `delete` under the image list. Selecting an image is an Application
 * Preference write and is attributed by its setting action
 * (`selectedBackgroundImg`) instead.
 */
export type BackgroundImageAction = "upload" | "delete";

/**
 * The background image actions whose last run failed, held per action. The
 * hooks are mounted from several places, so the state is shared here and each
 * control renders the action it owns. Only the next success of the same
 * action clears its own entry, and one action failing never hides another.
 * Automatic reads (the list and the selected image at startup) log only. The
 * backend detail goes to the log, never to the UI.
 */
export const backgroundImageFailuresAtom = atom<
  ReadonlySet<BackgroundImageAction>
>(new Set<BackgroundImageAction>());
