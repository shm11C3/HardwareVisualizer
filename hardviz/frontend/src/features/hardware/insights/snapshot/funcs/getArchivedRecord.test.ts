import { beforeEach, describe, expect, it, vi } from "vitest";

const hoisted = vi.hoisted(() => ({
  getDataArchiveSeriesMock: vi.fn(),
  getProcessStatsInPeriodMock: vi.fn(),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getDataArchiveSeries: hoisted.getDataArchiveSeriesMock,
    getProcessStatsInPeriod: hoisted.getProcessStatsInPeriodMock,
  },
}));

describe("getArchivedRecord functions", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe("getArchivedRecord", () => {
    it("queries DATA_ARCHIVE for cpu with correct time range", async () => {
      const start = new Date("2023-06-01T00:00:00.000Z");
      const end = new Date("2023-06-01T01:00:00.000Z");
      const mockRows = [
        {
          value: 45.2,
          timestamp: new Date("2023-06-01T00:30:00.000Z").getTime(),
        },
      ];
      hoisted.getDataArchiveSeriesMock.mockResolvedValue({
        status: "ok",
        data: mockRows,
      });

      const { getArchivedRecord } = await import(
        "@/features/hardware/insights/snapshot/funcs/getArchivedRecord"
      );
      const result = await getArchivedRecord("cpu", start, end, 60_000);

      expect(hoisted.getDataArchiveSeriesMock).toHaveBeenCalledWith(
        "cpu",
        "avg",
        start.toISOString(),
        end.toISOString(),
        60_000,
        "start",
      );
      expect(result).toEqual(mockRows);
    });

    it("queries DATA_ARCHIVE for ram", async () => {
      hoisted.getDataArchiveSeriesMock.mockResolvedValue({
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
        60_000,
      );

      expect(hoisted.getDataArchiveSeriesMock).toHaveBeenCalledWith(
        "memory",
        "avg",
        "2023-06-01T00:00:00.000Z",
        "2023-06-01T01:00:00.000Z",
        60_000,
        "start",
      );
    });

    it("throws when the archive command returns an error result", async () => {
      hoisted.getDataArchiveSeriesMock.mockResolvedValue({
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
          60_000,
        ),
      ).rejects.toThrow(
        "Failed to fetch archived hardware series: archive failed",
      );
    });
  });

  describe("getProcessStatsInPeriod", () => {
    it("passes the ISO start and end to the period command and returns its rows", async () => {
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
