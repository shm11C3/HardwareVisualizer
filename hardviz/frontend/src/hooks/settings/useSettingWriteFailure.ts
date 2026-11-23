import { useAtomValue } from "jotai";
import {
  type SettingWriteAction,
  settingsWriteFailuresAtom,
} from "@/store/settings";

/**
 * Whether a write failed for one of the `actions` the calling control owns.
 * Pass exactly the ids of the control the line sits under; a later successful
 * write of the same action clears it.
 */
export const useSettingWriteFailure = (
  actions: SettingWriteAction | readonly SettingWriteAction[],
): boolean => {
  const failures = useAtomValue(settingsWriteFailuresAtom);

  return typeof actions === "string"
    ? failures.has(actions)
    : actions.some((action) => failures.has(action));
};
