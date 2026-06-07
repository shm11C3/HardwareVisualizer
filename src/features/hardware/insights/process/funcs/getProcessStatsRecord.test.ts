import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  getProcessStatsMock: vi.fn(),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getProcessStats: hoisted.getProcessStatsMock,
  },
}));

describe("getProcessStats (getProcessStatsRecord)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("calls the typed process stats command with the period and end time", async () => {
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
    hoisted.getProcessStatsMock.mockResolvedValue({
      status: "ok",
      data: expectedRows,
    });

    const { getProcessStats } = await import(
      "@/features/hardware/insights/process/funcs/getProcessStatsRecord"
    );

    const result = await getProcessStats(period, endAt);

    expect(hoisted.getProcessStatsMock).toHaveBeenCalledWith(
      period,
      endAt.toISOString(),
    );
    expect(result).toEqual(expectedRows);
  });

  it("normalizes a string period before calling the typed command", async () => {
    const endAt = new Date("2023-06-01T01:00:00.000Z");
    hoisted.getProcessStatsMock.mockResolvedValue({
      status: "ok",
      data: [],
    });

    const { getProcessStats } = await import(
      "@/features/hardware/insights/process/funcs/getProcessStatsRecord"
    );

    await getProcessStats("180", endAt);

    expect(hoisted.getProcessStatsMock).toHaveBeenCalledWith(
      180,
      endAt.toISOString(),
    );
  });

  it("returns empty array when the command resolves with []", async () => {
    hoisted.getProcessStatsMock.mockResolvedValue({
      status: "ok",
      data: [],
    });

    const { getProcessStats } = await import(
      "@/features/hardware/insights/process/funcs/getProcessStatsRecord"
    );

    const result = await getProcessStats(5, new Date());
    expect(result).toEqual([]);
  });
});
