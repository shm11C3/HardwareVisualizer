import { Channel } from "@tauri-apps/api/core";
import { useEffect, useMemo, useState } from "react";
import { commands, type DownloadEvent } from "@/rspc/bindings";
import { isOk } from "@/types/result";

type UpdateMeta = {
  version: string;
  currentVersion: string;
  notes?: string | null;
  pubDate?: string | null;
};

export function useUpdater() {
  const [meta, setMeta] = useState<UpdateMeta | null>(null);

  const [installing, setInstalling] = useState(false);
  const [downloaded, setDownloaded] = useState<bigint>(0n);
  const [total, setTotal] = useState<bigint | null>(null);
  const [isFinished, setIsFinished] = useState(false);

  const percent = useMemo(() => {
    if (!total || total === 0n) return null;
    // 0..100
    const p = (downloaded * 100n) / total;
    const n = Number(p);
    return Number.isFinite(n) ? Math.max(0, Math.min(100, n)) : null;
  }, [downloaded, total]);

  useEffect(() => {
    (async () => {
      const res = await commands.fetchUpdate();
      if (isOk(res)) {
        setMeta(res.data);
      }
    })();
  }, []);

  const install = async () => {
    setInstalling(true);
    setDownloaded(0n);
    setTotal(null);

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
          setIsFinished(true);
          break;
        }
      }
    };

    await commands.installUpdate(ch);
    await commands.restartApp();
  };

  return {
    meta,
    installing,
    percent,
    downloaded,
    total,
    install,
    isFinished,
  };
}
