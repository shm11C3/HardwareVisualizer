import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  getProcessStatsMock: vi.fn(),
  getDataArchiveRecordsMock: vi.fn(),
  getProcessStatsInPeriodMock: vi.fn(),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getProcessStats: hoisted.getProcessStatsMock,
    getDataArchiveRecords: hoisted.getDataArchiveRecordsMock,
    getProcessStatsInPeriod: hoisted.getProcessStatsInPeriodMock,
  },
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
      hoisted.getProcessStatsMock.mockResolvedValue({
        status: "ok",
        data: mockRows,
      });

      const { getProcessStats } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getProcessStats(period, endAt);

      expect(hoisted.getProcessStatsMock).toHaveBeenCalledWith(
        period,
        endAt.toISOString(),
      );
      expect(result).toEqual(mockRows);
    });

    it("normalizes a string period before calling the typed command", async () => {
      const endAt = new Date("2023-06-01T02:00:00.000Z");
      hoisted.getProcessStatsMock.mockResolvedValue({
        status: "ok",
        data: [],
      });

      const { getProcessStats } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      await getProcessStats("180", endAt);

      expect(hoisted.getProcessStatsMock).toHaveBeenCalledWith(
        180,
        endAt.toISOString(),
      );
    });

    it("throws when the process stats command returns an error result", async () => {
      hoisted.getProcessStatsMock.mockResolvedValue({
        status: "error",
        error: "process stats failed",
      });

      const { getProcessStats } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );

      await expect(getProcessStats(30, new Date())).rejects.toThrow(
        "Failed to fetch process stats: process stats failed",
      );
    });
  });

  describe("getArchivedRecord", () => {
    it("queries DATA_ARCHIVE for cpu with correct time range", async () => {
      const start = new Date("2023-06-01T00:00:00.000Z");
      const end = new Date("2023-06-01T01:00:00.000Z");
      const mockRows = [
        { id: 1, value: 45.2, timestamp: "2023-06-01T00:30:00.000Z" },
      ];
      hoisted.getDataArchiveRecordsMock.mockResolvedValue({
        status: "ok",
        data: mockRows,
      });

      const { getArchivedRecord } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getArchivedRecord("cpu", start, end);

      expect(hoisted.getDataArchiveRecordsMock).toHaveBeenCalledWith(
        "cpu",
        "avg",
        start.toISOString(),
        end.toISOString(),
      );
      expect(result).toEqual(mockRows);
    });

    it("queries DATA_ARCHIVE for ram", async () => {
      hoisted.getDataArchiveRecordsMock.mockResolvedValue({
        status: "ok",
        data: [],
      });

      const { getArchivedRecord } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      await getArchivedRecord(
        "ram",
        new Date("2023-06-01T00:00:00.000Z"),
        new Date("2023-06-01T01:00:00.000Z"),
      );

      expect(hoisted.getDataArchiveRecordsMock).toHaveBeenCalledWith(
        "memory",
        "avg",
        "2023-06-01T00:00:00.000Z",
        "2023-06-01T01:00:00.000Z",
      );
    });

    it("throws when the archive command returns an error result", async () => {
      hoisted.getDataArchiveRecordsMock.mockResolvedValue({
        status: "error",
        error: "archive failed",
      });

      const { getArchivedRecord } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );

      await expect(
        getArchivedRecord(
          "cpu",
          new Date("2023-06-01T00:00:00.000Z"),
          new Date("2023-06-01T01:00:00.000Z"),
        ),
      ).rejects.toThrow(
        "Failed to fetch archived hardware records: archive failed",
      );
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
      hoisted.getProcessStatsInPeriodMock.mockResolvedValue({
        status: "ok",
        data: mockRows,
      });

      const { getProcessStatsInPeriod } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getProcessStatsInPeriod(start, end);

      expect(hoisted.getProcessStatsInPeriodMock).toHaveBeenCalledWith(
        start.toISOString(),
        end.toISOString(),
      );
      expect(result).toEqual(mockRows);
    });

    it("returns empty array when no records found", async () => {
      hoisted.getProcessStatsInPeriodMock.mockResolvedValue({
        status: "ok",
        data: [],
      });

      const { getProcessStatsInPeriod } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getProcessStatsInPeriod(new Date(), new Date());

      expect(hoisted.getProcessStatsInPeriodMock).toHaveBeenCalledOnce();
      expect(result).toEqual([]);
    });

    it("throws when the process stats period command returns an error result", async () => {
      hoisted.getProcessStatsInPeriodMock.mockResolvedValue({
        status: "error",
        error: "period failed",
      });

      const { getProcessStatsInPeriod } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );

      await expect(
        getProcessStatsInPeriod(new Date(), new Date()),
      ).rejects.toThrow(
        "Failed to fetch process stats in period: period failed",
      );
    });
  });
});
