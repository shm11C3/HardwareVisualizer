import { atom } from "jotai";
import type { UpdateMetadata } from "@/rspc/bindings";

/** The release the latest update check found, or null when none is known. */
export const availableUpdateAtom = atom<UpdateMetadata | null>(null);

/** Whether the update confirmation dialog is open. */
export const updateDialogOpenAtom = atom(false);

/** Set once the user starts installing; cleared when it fails before shutdown. */
export const updateInstallStartedAtom = atom(false);

/**
 * The version the side menu offers, or null while nothing is available or an
 * install is already under way.
 */
export const updateIndicatorVersionAtom = atom((get) => {
  const update = get(availableUpdateAtom);
  return update && !get(updateInstallStartedAtom) ? update.version : null;
});
