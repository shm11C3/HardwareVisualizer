import { useInsightChart } from "@/features/hardware/insights/components/useInsightChart";
import { sqlitePromise } from "@/lib/sqlite";
import { act, renderHook } from "@testing-library/react";
import { type Mock, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/sqlite", () => ({
  sqlitePromise: Promise.resolve({
    load: vi.fn().mockResolvedValue([]),
  }),
}));

vi.mock("@/consts", () => ({
  chartConfig: {
    archiveUpdateIntervalMilSec: 60000,
  },
}));

describe("useInsightChart", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should fetch and aggregate data for non-GPU hardware", async () => {
    const mockData = [
      { value: 10, timestamp: "2023-01-01T00:00:00Z" },
      { value: 20, timestamp: "2023-01-01T00:01:00Z" },
    ];
    ((await sqlitePromise).load as Mock).mockResolvedValue(mockData);

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels).toHaveLength(11);
    expect(result.current.chartData).toContain(10);
    expect(result.current.chartData).toContain(20);
  });

  it("should fetch and aggregate memory max", async () => {
    const mockData = [
      { value: 2000, timestamp: "2023-01-01T00:00:00Z" },
      { value: 3000, timestamp: "2023-01-01T00:01:00Z" },
    ];
    ((await sqlitePromise).load as Mock).mockResolvedValue(mockData);

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "memory",
        dataStats: "max",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.chartData).toContain(3000);
  });

  it("should fetch and aggregate data for GPU hardware", async () => {
    const mockData = [
      { value: 30, timestamp: "2023-01-01T00:00:00Z" },
      { value: 40, timestamp: "2023-01-01T00:01:00Z" },
    ];
    ((await sqlitePromise).load as Mock).mockResolvedValue(mockData);

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "gpu",
        dataStats: "max",
        dataType: "usage",
        period: 10,
        offset: 0,
        gpuName: "NVIDIA",
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels).toHaveLength(11);
    expect(result.current.chartData).toContain(40); // Max of mockData
  });

  it("should fetch and aggregate GPU temperature with min", async () => {
    const mockData = [
      { value: 60, timestamp: "2023-01-01T00:00:00Z" },
      { value: 50, timestamp: "2023-01-01T00:01:00Z" },
    ];
    ((await sqlitePromise).load as Mock).mockResolvedValue(mockData);

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "gpu",
        dataStats: "min",
        dataType: "temp",
        period: 10,
        offset: 0,
        gpuName: "Intel",
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.chartData).toContain(50);
  });

  const mockedTime = new Date("2023-01-01T00:02:00Z");
  vi.setSystemTime(mockedTime);

  it("should handle empty data gracefully", async () => {
    ((await sqlitePromise).load as Mock).mockResolvedValue([]);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "memory",
        dataStats: "min",
        period: 10,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels).toHaveLength(11);
    expect(result.current.chartData).toEqual(Array(11).fill(null));
  });

  it("should calculate labels correctly for long periods", async () => {
    ((await sqlitePromise).load as Mock).mockResolvedValue([]);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 1440,
        offset: 0,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels).toBeDefined();
    expect(result.current.labels[0]).toMatch(/\d{4}/); // Year should be included
  });

  it("should shift time correctly when offset is applied", async () => {
    const mockData = [{ value: 15, timestamp: "2023-01-01T00:00:00Z" }];
    ((await sqlitePromise).load as Mock).mockResolvedValue(mockData);

    const mockedTime = new Date("2023-01-01T00:02:00Z");
    vi.setSystemTime(mockedTime);

    const { result } = renderHook(() =>
      useInsightChart({
        hardwareType: "cpu",
        dataStats: "avg",
        period: 10,
        offset: 5,
      }),
    );

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    expect(result.current.labels.length).toBeGreaterThan(0);
  });
});
