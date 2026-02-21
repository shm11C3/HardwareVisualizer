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

describe("getArchivedRecord functions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("getProcessStats", () => {
    it("queries process_stats with the correct time window", async () => {
      const endAt = new Date("2023-06-01T02:00:00.000Z");
      const period = 30;
      const mockRows = [
        {
          pid: 1,
          process_name: "foo",
          avg_cpu_usage: 5,
          avg_memory_usage: 100,
          total_execution_sec: 60,
          latest_timestamp: "2023-06-01T02:00:00.000Z",
        },
      ];
      hoisted.loadMock.mockResolvedValue(mockRows);

      const { getProcessStats } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getProcessStats(period, endAt);

      expect(hoisted.loadMock).toHaveBeenCalledOnce();
      const sql = hoisted.loadMock.mock.calls[0][0] as string;
      expect(sql).toContain("process_stats");
      // adjustedEndAt = endAt - 60000ms = 2023-06-01T01:59:00.000Z
      // startTime = adjustedEndAt - 30*60*1000 = 2023-06-01T01:29:00.000Z
      expect(sql).toContain("2023-06-01T01:29:00.000Z");
      expect(sql).toContain("2023-06-01T01:59:00.000Z");
      expect(result).toEqual(mockRows);
    });
  });

  describe("getArchivedRecord", () => {
    it("queries DATA_ARCHIVE for cpu with correct time range", async () => {
      const start = new Date("2023-06-01T00:00:00.000Z");
      const end = new Date("2023-06-01T01:00:00.000Z");
      const mockRows = [
        { id: 1, value: 45.2, timestamp: "2023-06-01T00:30:00.000Z" },
      ];
      hoisted.loadMock.mockResolvedValue(mockRows);

      const { getArchivedRecord } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getArchivedRecord("cpu", start, end);

      expect(hoisted.loadMock).toHaveBeenCalledOnce();
      const sql = hoisted.loadMock.mock.calls[0][0] as string;
      expect(sql).toContain("cpu_avg");
      expect(sql).toContain("DATA_ARCHIVE");
      expect(sql).toContain(start.toISOString());
      expect(sql).toContain(end.toISOString());
      expect(result).toEqual(mockRows);
    });

    it("queries DATA_ARCHIVE for ram", async () => {
      hoisted.loadMock.mockResolvedValue([]);

      const { getArchivedRecord } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      await getArchivedRecord(
        "ram",
        new Date("2023-06-01T00:00:00.000Z"),
        new Date("2023-06-01T01:00:00.000Z"),
      );

      const sql = hoisted.loadMock.mock.calls[0][0] as string;
      expect(sql).toContain("ram_avg");
    });
  });

  describe("getProcessStatsInPeriod", () => {
    it("queries process_stats between start and end with ORDER BY", async () => {
      const start = new Date("2023-06-01T00:00:00.000Z");
      const end = new Date("2023-06-01T01:00:00.000Z");
      const mockRows = [
        {
          pid: 10,
          process_name: "bar",
          avg_cpu_usage: 20,
          avg_memory_usage: 256,
          total_execution_sec: 120,
          latest_timestamp: "2023-06-01T00:50:00.000Z",
        },
      ];
      hoisted.loadMock.mockResolvedValue(mockRows);

      const { getProcessStatsInPeriod } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getProcessStatsInPeriod(start, end);

      expect(hoisted.loadMock).toHaveBeenCalledOnce();
      const sql = hoisted.loadMock.mock.calls[0][0] as string;
      expect(sql).toContain("process_stats");
      expect(sql).toContain(start.toISOString());
      expect(sql).toContain(end.toISOString());
      expect(sql).toContain("ORDER BY");
      expect(result).toEqual(mockRows);
    });

    it("returns empty array when no records found", async () => {
      hoisted.loadMock.mockResolvedValue([]);

      const { getProcessStatsInPeriod } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getProcessStatsInPeriod(new Date(), new Date());

      expect(hoisted.loadMock).toHaveBeenCalledOnce();
      expect(result).toEqual([]);
    });
  });
});
