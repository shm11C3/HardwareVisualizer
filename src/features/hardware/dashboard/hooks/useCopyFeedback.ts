import { useCallback, useEffect, useRef, useState } from "react";

const COPIED_DURATION_MS = 2000;

/**
 * Runs a copy action and reports its outcome: `copied` is true for a short
 * moment after success, `failed` stays true until the next attempt. Only the
 * latest attempt may update the outcome, so an older attempt that settles
 * late cannot leave success and failure shown together.
 */
export const useCopyFeedback = (copy: () => Promise<void>) => {
  const [copied, setCopied] = useState(false);
  const [failed, setFailed] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const latestAttempt = useRef(0);

  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current);
    },
    [],
  );

  const run = useCallback(async () => {
    latestAttempt.current += 1;
    const attempt = latestAttempt.current;
    if (timer.current !== null) clearTimeout(timer.current);
    setCopied(false);
    setFailed(false);
    let succeeded = true;
    try {
      await copy();
    } catch {
      succeeded = false;
    }
    if (attempt !== latestAttempt.current) return;
    if (!succeeded) {
      setFailed(true);
      return;
    }
    setCopied(true);
    timer.current = setTimeout(() => setCopied(false), COPIED_DURATION_MS);
  }, [copy]);

  return { copied, failed, copy: run };
};
