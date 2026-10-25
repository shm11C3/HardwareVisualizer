import { describe, expect, it } from "vitest";
import { getProcessBubbleData } from "@/features/hardware/insights/process/chart/getProcessBubbleData";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";

const record = (
  pid: number,
  overrides: Partial<ProcessStat> = {},
): ProcessStat => ({
  pid,
  process_name: `process-${pid}`,
  avg_cpu_usage: 0.01,
  avg_memory_usage: 1,
  total_execution_sec: 60,
  latest_timestamp: "2026-10-15T16:14:12.661Z",
  ...overrides,
});

const rankedRecords = () =>
  Array.from({ length: 750 }, (_, i) =>
    record(i + 1, {
      avg_cpu_usage: i < 250 ? 80 : 0.01,
      avg_memory_usage: i >= 250 && i < 500 ? 2_000_000 : 1,
    }),
  );

describe("getProcessBubbleData", () => {
  it("keeps the CPU and memory leaders within 500 points without mutating the input", () => {
    const records = rankedRecords();
    records[0].total_execution_sec = 1;
    const before = structuredClone(records);
    const result = getProcessBubbleData(records);

    expect(result.data).toHaveLength(500);
    expect(result.data.map((point) => point.pid)).toEqual(
      records.slice(0, 500).map((stat) => stat.pid),
    );
    expect(result.data[0]).toMatchObject({ x: 1 / 60, y: 80 });
    expect(result.data[250].ram).toBe(2_000_000);
    expect(result.omittedCount).toBe(250);
    expect(result.allData).toHaveLength(750);
    expect(records).toEqual(before);
  });

  it("preserves distinct archive groups when a PID is reused for another process name", () => {
    const records = rankedRecords();
    records[0].process_name = "cpu-worker";
    records[250].pid = records[0].pid;
    records[250].process_name = "memory-worker";
    const result = getProcessBubbleData(records);

    expect(
      result.data
        .filter((point) => point.pid === records[0].pid)
        .map((point) => point.name),
    ).toEqual(["cpu-worker", "memory-worker"]);
    expect(result.data).toHaveLength(500);
  });

  it("counts overlapping CPU and memory leaders once", () => {
    const records = rankedRecords();
    records.forEach((stat, i) => {
      stat.avg_memory_usage = i < 250 ? 2_000_000 : 1;
    });
    const result = getProcessBubbleData(records);

    expect(result.data).toHaveLength(250);
    expect(result.omittedCount).toBe(500);
  });

  it("breaks metric ties by archived identity instead of input order", () => {
    const records = Array.from({ length: 600 }, (_, i) => record(i + 1));
    const selected = getProcessBubbleData(records).data.map(
      (point) => point.pid,
    );
    const reversed = getProcessBubbleData([...records].reverse())
      .data.map((point) => point.pid)
      .sort((a, b) => a - b);

    expect(selected).toEqual(Array.from({ length: 250 }, (_, i) => i + 1));
    expect(reversed).toEqual(selected);
  });

  it("retains omitted points for zoom and size scales without using lifetime to rank them", () => {
    const records = rankedRecords();
    records[500].total_execution_sec = 1_200_000;
    const result = getProcessBubbleData(records);

    expect(result.data.some((point) => point.pid === records[500].pid)).toBe(
      false,
    );
    expect(result.maxX).toBe(20_000);
    expect(result.maxZ).toBe(Math.sqrt(12_000));
    expect(result.allData[500]).toMatchObject({
      x: 20_000,
      z: Math.sqrt(12_000),
    });
  });

  it("keeps small results complete and handles an empty result", () => {
    const result = getProcessBubbleData([record(42), record(43)]);
    expect(result.data).toEqual(result.allData);
    expect(result.omittedCount).toBe(0);
    expect(getProcessBubbleData([])).toEqual({
      allData: [],
      data: [],
      omittedCount: 0,
      maxX: 0,
      maxZ: 0,
    });
  });
});
