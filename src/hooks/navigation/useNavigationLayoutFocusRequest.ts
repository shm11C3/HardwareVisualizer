import { useAtom, useSetAtom } from "jotai";
import { useCallback } from "react";
import { navigationLayoutFocusRequestedAtom } from "@/store/navigation";

/**
 * Asks the Navigation Layout toggle in Settings to take keyboard focus.
 * Write-only: callers that only raise the request do not subscribe to it.
 */
export const useRequestNavigationLayoutFocus = () => {
  const setFocusRequested = useSetAtom(navigationLayoutFocusRequestedAtom);
  const requestFocus = useCallback(
    () => setFocusRequested(true),
    [setFocusRequested],
  );
  return { requestFocus };
};

/**
 * The consuming side of the focus request: whether one is pending, and how to
 * clear it once the toggle has taken focus.
 */
export const useNavigationLayoutFocusRequest = () => {
  const [focusRequested, setFocusRequested] = useAtom(
    navigationLayoutFocusRequestedAtom,
  );
  const clearFocusRequest = useCallback(
    () => setFocusRequested(false),
    [setFocusRequested],
  );
  return { focusRequested, clearFocusRequest };
};
