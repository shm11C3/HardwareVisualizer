import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatBytesBigint,
  formatDuration,
} from "@/lib/formatter";

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

describe("formatBytesBigint", () => {
  it("should return empty string for undefined", () => {
    expect(formatBytesBigint()).toBe("");
  });

  it("should format bytes correctly", () => {
    expect(formatBytesBigint(0n)).toBe("0.0 B");
    expect(formatBytesBigint(16n)).toBe("16.0 B");
    expect(formatBytesBigint(1024n)).toBe("1.0 KB");
    expect(formatBytesBigint(1024n ** 2n)).toBe("1.0 MB");
    expect(formatBytesBigint(1024n ** 3n)).toBe("1.0 GB");
    expect(formatBytesBigint(1024n ** 4n)).toBe("1.0 TB");
  });
});
