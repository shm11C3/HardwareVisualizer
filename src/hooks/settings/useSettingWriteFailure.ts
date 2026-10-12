import { useAtomValue } from "jotai";
import type { ClientSettings } from "@/rspc/bindings";
import { settingsWriteFailureAtom } from "@/store/settings";

/**
 * Whether the last Application Preference write failed for one of `keys`.
 * The control that owns the setting renders the failure under itself; a later
 * successful write of the same key clears it.
 */
export const useSettingWriteFailure = (
  keys: keyof ClientSettings | readonly (keyof ClientSettings)[],
): boolean => {
  const failure = useAtomValue(settingsWriteFailureAtom);

  if (failure === null) return false;
  return typeof keys === "string"
    ? failure.key === keys
    : keys.includes(failure.key);
};
