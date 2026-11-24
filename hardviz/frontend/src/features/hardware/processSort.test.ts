import { describe, expect, it } from "vitest";
import { sortProcesses } from "@/features/hardware/processSort";
import type { ProcessInfo } from "@/rspc/bindings";

const proc = (pid: number, gpuUsage: string | null): ProcessInfo => ({
  pid,
  name: `p${pid}`,
  cpuUsage: "0",
  memoryUsage: "0",
  gpuUsage,
});

describe("sortProcesses by gpuUsage", () => {
  const processes = [
    proc(1, null),
    proc(2, "10"),
    proc(3, "130.5"),
    proc(4, "2"),
  ];

  it("puts unavailable readings last when ascending", () => {
    const sorted = sortProcesses(processes, {
      key: "gpuUsage",
      direction: "ascending",
    });
    expect(sorted.map((p) => p.pid)).toEqual([4, 2, 3, 1]);
  });

  it("puts unavailable readings last when descending", () => {
    const sorted = sortProcesses(processes, {
      key: "gpuUsage",
      direction: "descending",
    });
    expect(sorted.map((p) => p.pid)).toEqual([3, 2, 4, 1]);
  });
});
