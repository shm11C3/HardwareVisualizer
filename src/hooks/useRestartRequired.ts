import { useAtom, useSetAtom } from "jotai";
import { useCallback } from "react";
import { isRestartRequiredAtom } from "@/store/ui";

/**
 * Marks that a setting change only takes effect after the app restarts.
 * Write-only: callers that only raise the flag do not subscribe to it.
 */
export const useMarkRestartRequired = () => {
  const setIsRestartRequired = useSetAtom(isRestartRequiredAtom);
  const markRestartRequired = useCallback(
    () => setIsRestartRequired(true),
    [setIsRestartRequired],
  );
  return { markRestartRequired };
};

/** Whether a restart is pending, together with the way to mark one. */
export const useRestartRequired = () => {
  const [isRestartRequired, setIsRestartRequired] = useAtom(
    isRestartRequiredAtom,
  );
  const markRestartRequired = useCallback(
    () => setIsRestartRequired(true),
    [setIsRestartRequired],
  );
  return { isRestartRequired, markRestartRequired };
};
