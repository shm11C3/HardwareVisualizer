import { Channel } from "@tauri-apps/api/core";
import { useEffect, useMemo, useRef, useState } from "react";
import {
  commands,
  type DownloadEvent,
  type UpdaterError,
} from "@/rspc/bindings";
import { isOk } from "@/types/result";

type UpdateMeta = {
  version: string;
  currentVersion: string;
  notes?: string | null;
  pubDate?: string | null;
};

/**
 * How often a running app looks for a newer release. The app can stay resident
 * in the tray for days, so a startup-only check would leave it on an old
 * version until the next launch.
 */
export const UPDATE_CHECK_INTERVAL_MS = 6 * 60 * 60 * 1000;

export type UpdateInstallError =
  | { kind: "before-shutdown"; message: string }
  | { kind: "restart-required"; message: string };

function installErrorFrom(error: UpdaterError): UpdateInstallError {
  if (error === "NoPendingUpdate") {
    return {
      kind: "before-shutdown",
      message: "NoPendingUpdate",
    };
  }
  if ("RestartRequired" in error) {
    return {
      kind: "restart-required",
      message: error.RestartRequired,
    };
  }
  return {
    kind: "before-shutdown",
    message: error.Updater,
  };
}

/**
 * Keep the current meta while the recheck finds the same version: a new object
 * would reopen the update modal the user already postponed with "Later".
 */
function nextMeta(
  current: UpdateMeta | null,
  fetched: UpdateMeta | null,
): UpdateMeta | null {
  return current && fetched && current.version === fetched.version
    ? current
    : fetched;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function useUpdater() {
  const [meta, setMeta] = useState<UpdateMeta | null>(null);
  // install_update consumes the pending update; a concurrent check must not
  // replace it or reopen the modal mid-install.
  const installStartedRef = useRef(false);

  const [installing, setInstalling] = useState(false);
  const [downloaded, setDownloaded] = useState<bigint>(0n);
  const [total, setTotal] = useState<bigint | null>(null);
  const [isFinished, setIsFinished] = useState(false);
  const [installError, setInstallError] = useState<UpdateInstallError | null>(
    null,
  );

  const percent = useMemo(() => {
    if (!total || total === 0n) return null;
    // 0..100
    const p = (downloaded * 100n) / total;
    const n = Number(p);
    return Number.isFinite(n) ? Math.max(0, Math.min(100, n)) : null;
  }, [downloaded, total]);

  useEffect(() => {
    const check = async () => {
      if (installStartedRef.current) return;
      const res = await commands.fetchUpdate();
      if (isOk(res)) {
        setMeta((current) => nextMeta(current, res.data));
      }
    };

    void check();
    const intervalId = window.setInterval(check, UPDATE_CHECK_INTERVAL_MS);
    return () => window.clearInterval(intervalId);
  }, []);

  const install = async () => {
    installStartedRef.current = true;
    setInstalling(true);
    setDownloaded(0n);
    setTotal(null);
    setIsFinished(false);
    setInstallError(null);
    let downloadCompleted = false;

    const ch: Channel<DownloadEvent> = new Channel<DownloadEvent>();
    ch.onmessage = (e) => {
      switch (e.event) {
        case "started": {
          const s = e.data.contentLength;
          setTotal(s ? BigInt(s) : null);
          break;
        }
        case "progress": {
          setDownloaded((prev) => prev + BigInt(e.data.chunkLength));
          break;
        }
        case "finished": {
          downloadCompleted = true;
          break;
        }
      }
    };

    try {
      const result = await commands.installUpdate(ch);
      if (isOk(result)) {
        setIsFinished(true);
        try {
          await commands.restartApp();
        } catch (error) {
          setInstalling(false);
          setInstallError({
            kind: "restart-required",
            message: errorMessage(error),
          });
        }
        return;
      }

      const failure = installErrorFrom(result.error);
      setInstalling(false);
      setIsFinished(false);
      setInstallError(failure);

      if (failure.kind === "before-shutdown") {
        installStartedRef.current = false;
        // install_update consumes PendingUpdate before downloading. Recheck
        // after a download failure so the update button can retry with a new
        // pending value, without restarting an app whose workers are running.
        try {
          const refreshed = await commands.fetchUpdate();
          if (isOk(refreshed)) {
            setMeta(refreshed.data);
          }
        } catch {
          // Preserve the install error; the modal remains available to close.
        }
      }
    } catch (error) {
      const message = errorMessage(error);
      const kind = downloadCompleted ? "restart-required" : "before-shutdown";
      installStartedRef.current = kind === "restart-required";
      setInstalling(false);
      setIsFinished(false);
      setInstallError({ kind, message });
    }
  };

  return {
    meta,
    installing,
    percent,
    downloaded,
    total,
    install,
    isFinished,
    installError,
  };
}
