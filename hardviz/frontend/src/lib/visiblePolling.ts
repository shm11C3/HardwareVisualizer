/**
 * Run `poll` while the document is visible and stop while it is hidden.
 *
 * `poll` also runs immediately whenever polling starts, so a surface that
 * becomes visible again shows fresh data instead of the value it was hidden
 * with.
 *
 * The interval is cleared rather than left to the platform: a window hidden to
 * the tray keeps its WebView resident, and WebKit throttles hidden-page timers
 * only down to a ~1s floor, which does not slow an interval that is already
 * longer than that.
 *
 * Callers keep owning cancellation of their own in-flight requests.
 */
export const startVisiblePolling = (
  poll: () => void,
  intervalMs: number,
): (() => void) => {
  let intervalId: number | undefined;

  const stopInterval = () => {
    if (intervalId === undefined) {
      return;
    }

    window.clearInterval(intervalId);
    intervalId = undefined;
  };

  const startInterval = () => {
    if (intervalId !== undefined) {
      return;
    }

    poll();
    intervalId = window.setInterval(poll, intervalMs);
  };

  const handleVisibilityChange = () => {
    if (document.hidden) {
      stopInterval();
      return;
    }

    startInterval();
  };

  if (!document.hidden) {
    startInterval();
  }

  document.addEventListener("visibilitychange", handleVisibilityChange);

  return () => {
    document.removeEventListener("visibilitychange", handleVisibilityChange);
    stopInterval();
  };
};
