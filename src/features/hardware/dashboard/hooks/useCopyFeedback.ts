import { useCallback, useEffect, useRef, useState } from "react";

const COPIED_DURATION_MS = 2000;

/**
 * Runs a copy action and reports its outcome: `copied` is true for a short
 * moment after success, `failed` stays true until the next attempt.
 */
export const useCopyFeedback = (copy: () => Promise<void>) => {
  const [copied, setCopied] = useState(false);
  const [failed, setFailed] = useState(false);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current);
    },
    [],
  );

  const run = useCallback(async () => {
    if (timer.current !== null) clearTimeout(timer.current);
    setFailed(false);
    try {
      await copy();
    } catch {
      setCopied(false);
      setFailed(true);
      return;
    }
    setCopied(true);
    timer.current = setTimeout(() => setCopied(false), COPIED_DURATION_MS);
  }, [copy]);

  return { copied, failed, copy: run };
};
