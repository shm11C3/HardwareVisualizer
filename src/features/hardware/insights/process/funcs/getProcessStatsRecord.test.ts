import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  loadMock: vi.fn(),
}));

vi.mock("@/lib/sqlite", () => ({
  sqlitePromise: Promise.resolve({
    load: hoisted.loadMock,
    save: vi.fn(),
  }),
}));

vi.mock("@/features/hardware/consts/chart", () => ({
  chartConfig: { archiveUpdateIntervalMilSec: 60000 },
}));

describe("getProcessStats (getProcessStatsRecord)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls db.load with a SQL query containing the correct time window", async () => {
    const endAt = new Date("2023-06-01T01:00:00.000Z");
    const period = 10; // 10 minutes
    const expectedRows = [
      {
        pid: 42,
        process_name: "node",
        avg_cpu_usage: 15,
        avg_memory_usage: 512,
        total_execution_sec: 600,
        latest_timestamp: "2023-06-01T01:00:00.000Z",
      },
    ];
    hoisted.loadMock.mockResolvedValue(expectedRows);

    const { getProcessStats } = await import(
      "@/features/hardware/insights/process/funcs/getProcessStatsRecord"
    );

    const result = await getProcessStats(period, endAt);

    expect(hoisted.loadMock).toHaveBeenCalledOnce();
    const sql = hoisted.loadMock.mock.calls[0][0] as string;
    expect(sql).toContain("process_stats");
    // adjustedEndAt = endAt - 60000ms; startTime = adjustedEndAt - 10*60*1000
    expect(sql).toContain("2023-06-01T00:49:00.000Z"); // startTime
    expect(sql).toContain("2023-06-01T00:59:00.000Z"); // adjustedEndAt
    expect(result).toEqual(expectedRows);
  });

  it("returns empty array when db.load resolves with []", async () => {
    hoisted.loadMock.mockResolvedValue([]);

    const { getProcessStats } = await import(
      "@/features/hardware/insights/process/funcs/getProcessStatsRecord"
    );

    const result = await getProcessStats(5, new Date());
    expect(result).toEqual([]);
  });
});
