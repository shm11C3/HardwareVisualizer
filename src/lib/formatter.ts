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

export const formatLocaleTime = (
  seconds: number,
  locale: Intl.LocalesArgument,
) => {
  return new Date(seconds * 1000).toLocaleTimeString(locale, {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    timeZone: "UTC",
  });
};
