import { formatBytes, formatDuration } from "@/lib/formatter";
import { describe, expect, it } from "vitest";

describe("formatBytes", () => {
  it("should return [0,'B'] for invalid numbers", () => {
    expect(formatBytes(-1)).toEqual([0, "B"]);
    expect(formatBytes(Number.NaN)).toEqual([0, "B"]);
  });

  it("should convert bytes to appropriate units", () => {
    expect(formatBytes(1024)).toEqual([1, "KB"]);
    expect(formatBytes(1024 ** 2)).toEqual([1, "MB"]);
    expect(formatBytes(1024 ** 3)).toEqual([1, "GB"]);
  });
});

describe("formatDuration", () => {
  it("should format duration in English", () => {
    expect(formatDuration(3661, "en-US")).toBe("1hour 1minute 1second");
  });

  it("should format duration in Japanese", () => {
    expect(formatDuration(90061, "ja-JP")).toBe("1日 1時間 1分 1秒");
  });
});
