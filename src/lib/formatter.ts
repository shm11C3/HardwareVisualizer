import type { SizeUnit } from "@/rspc/bindings";

export const formatBytes = (
  bytes: number,
  decimals = 2,
): [number, SizeUnit] => {
  if (!Number.isFinite(bytes) || bytes <= 0) return [0, "B"];

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes: SizeUnit[] = ["B", "KB", "MB", "GB"];

  const i = Math.min(
    Math.floor(Math.log(bytes) / Math.log(k)),
    sizes.length - 1,
  );

  return [Number.parseFloat((bytes / k ** i).toFixed(dm)), sizes[i]];
};

export const formatDuration = (seconds: number, locale: "ja-JP" | "en-US") => {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  const s = seconds % 60;

  const translations = {
    "ja-JP": { day: "日", hour: "時間", minute: "分", second: "秒" },
    "en-US": { day: "day", hour: "hour", minute: "minute", second: "second" },
  };

  const t = translations[locale];

  return `${d > 0 ? `${d}${t.day} ` : ""}${h > 0 ? `${h}${t.hour} ` : ""}${m > 0 ? `${m}${t.minute} ` : ""}${s}${t.second}`.trim();
};
