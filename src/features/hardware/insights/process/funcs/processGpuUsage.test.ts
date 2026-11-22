import { describe, expect, it } from "vitest";
import {
  gpuReadForWindow,
  joinProcessGpuUsage,
  processGpuCoverage,
  processStatsWindowStart,
} from "@/features/hardware/insights/process/funcs/processGpuUsage";
import type {
  ProcessGpuRead,
  ProcessStat,
} from "@/features/hardware/insights/types/processStats";
import type { ProcessGpuStatRecord } from "@/rspc/bindings";

const stat = (pid: number, process_name: string): ProcessStat => ({
  pid,
  process_name,
  avg_cpu_usage: 1,
  avg_memory_usage: 1024,
  total_execution_sec: 60,
  latest_timestamp: "2026-11-01T00:00:00+00:00",
});
const gpu = (
  pid: number,
  process_name: string,
  avg_gpu_usage: number,
): ProcessGpuStatRecord => ({
  pid,
  process_name,
  avg_gpu_usage,
  covered_seconds: 60,
});

describe("joinProcessGpuUsage", () => {
  it("joins by (pid, process_name) so a reused PID keeps its own GPU usage", () => {
    const rows = joinProcessGpuUsage(
      [stat(42, "game"), stat(42, "updater"), stat(7, "game")],
      [gpu(42, "updater", 3.5), gpu(42, "game", 88), gpu(7, "other", 9)],
    );

    expect(rows.map((row) => row.avg_gpu_usage)).toEqual([88, 3.5, null]);
  });

  it("keeps a real 0 apart from a missing record", () => {
    const rows = joinProcessGpuUsage(
      [stat(1, "idle"), stat(2, "cpu-only")],
      [gpu(1, "idle", 0)],
    );

    expect(rows.map((row) => row.avg_gpu_usage)).toEqual([0, null]);
  });

  it("keeps values above 100 as reported", () => {
    const [row] = joinProcessGpuUsage(
      [stat(1, "renderer")],
      [gpu(1, "renderer", 143.25)],
    );

    expect(row.avg_gpu_usage).toBe(143.25);
  });
});

describe("processStatsWindowStart", () => {
  it("mirrors the command window: one archive interval before endAt, then the period", () => {
    const endAt = new Date("2026-11-20T12:00:00.000Z");

    expect(processStatsWindowStart({ period: 60, endAt }).toISOString()).toBe(
      "2026-11-20T10:59:00.000Z",
    );
  });
});

describe("gpuReadForWindow", () => {
  const endAt = new Date("2026-11-20T12:00:00.000Z");
  const read = (period: number, at: Date): ProcessGpuRead => ({
    window: { period, endAt: at },
    status: "ready",
    stats: { kind: "available", records: [], recorded_since: null },
  });

  it("returns the GPU read for an equal window, even as a different Date instance", () => {
    const gpuRead = read(60, endAt);

    expect(
      gpuReadForWindow(gpuRead, { period: 60, endAt: new Date(endAt) }),
    ).toBe(gpuRead);
  });

  it("drops a GPU read left from another period or end time, in either order", () => {
    // The stats moved to the new period while GPU still answers the old one.
    expect(
      gpuReadForWindow(read(60, endAt), { period: 180, endAt }),
    ).toBeNull();
    // GPU moved first while the stats still answer the old one.
    expect(
      gpuReadForWindow(read(180, endAt), { period: 60, endAt }),
    ).toBeNull();
    expect(
      gpuReadForWindow(read(60, new Date(endAt.getTime() - 60_000)), {
        period: 60,
        endAt,
      }),
    ).toBeNull();
  });

  it("treats GPU as pending until both sides have a window", () => {
    expect(gpuReadForWindow(null, { period: 60, endAt })).toBeNull();
    expect(gpuReadForWindow(read(60, endAt), null)).toBeNull();
  });

  it("keeps a failure only for its own window", () => {
    const failed: ProcessGpuRead = {
      window: { period: 60, endAt },
      status: "failed",
    };

    expect(gpuReadForWindow(failed, { period: 60, endAt })).toBe(failed);
    expect(gpuReadForWindow(failed, { period: 180, endAt })).toBeNull();
  });
});

describe("processGpuCoverage", () => {
  const windowStart = new Date("2026-11-20T10:59:00.000Z");

  it("notes the first record when the period starts before it", () => {
    expect(
      processGpuCoverage("2026-11-20T11:30:00+00:00", windowStart),
    ).toEqual({
      kind: "recordedSince",
      recordedSince: new Date("2026-11-20T11:30:00Z"),
    });
  });

  it("needs no note when GPU usage was recorded for the whole period", () => {
    expect(
      processGpuCoverage("2026-11-19T00:00:00+00:00", windowStart),
    ).toEqual({ kind: "complete" });
    expect(
      processGpuCoverage("2026-11-20T10:59:00+00:00", windowStart),
    ).toEqual({ kind: "complete" });
  });

  it("reports that nothing was recorded yet when the archive has no GPU record", () => {
    expect(processGpuCoverage(null, windowStart)).toEqual({
      kind: "notRecordedYet",
    });
  });
});
