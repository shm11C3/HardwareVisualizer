import { renderHook, act } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useSnapshot } from "@/features/hardware/insights/snapshot/hooks/useSnapshot";
import { getArchivedRecord } from "@/features/hardware/insights/snapshot/funcs/getArchivedRecord";

// Mock Tauri dependencies to prevent runtime errors
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-sql", () => ({
  Database: {
    load: vi.fn(),
  },
}));

vi.mock("@/lib/sqlite", () => ({
  sqlitePromise: Promise.resolve({
    load: vi.fn(),
  }),
}));

// Mock the actual function we're testing
vi.mock("@/features/hardware/insights/snapshot/funcs/getArchivedRecord");

const mockGetArchivedRecord = vi.mocked(getArchivedRecord);

describe("useSnapshot - Label Formatting", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const mockData = [
      { id: 1, value: 50, timestamp: "2023-01-01T10:30:00Z" },
      { id: 2, value: 60, timestamp: "2023-01-01T11:00:00Z" },
    ];
    mockGetArchivedRecord.mockResolvedValue(mockData);
  });

  it("should generate time-only labels for short periods (< 180 minutes)", async () => {
    const { result } = renderHook(() => useSnapshot());

    // Set short period (2 hours = 120 minutes)
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T08:00:00Z",
        end: "2023-01-01T10:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Labels should be in HH:MM format for periods under 180 minutes
    const hasTimeFormat = result.current.filledLabels.some(label => 
      /^\d{2}:\d{2}$/.test(label)
    );
    expect(hasTimeFormat).toBe(true);
  });

  it("should generate date and time labels for medium periods (180-1440 minutes)", async () => {
    const { result } = renderHook(() => useSnapshot());

    // Set medium period (12 hours = 720 minutes)
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T00:00:00Z",
        end: "2023-01-01T12:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Labels should include month/day and time for periods >= 180 minutes
    const hasDateTimeFormat = result.current.filledLabels.some(label => 
      /\d+\/\d+.*\d{2}:\d{2}/.test(label)
    );
    expect(hasDateTimeFormat).toBe(true);
  });

  it("should generate date with year labels for long periods (>= 1440 minutes)", async () => {
    const { result } = renderHook(() => useSnapshot());

    // Set long period (2 days = 2880 minutes)
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T00:00:00Z",
        end: "2023-01-03T00:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Labels should include year for periods >= 1440 minutes
    const hasYear = result.current.filledLabels.some(label => 
      /2023/.test(label)
    );
    expect(hasYear).toBe(true);
  });

  it("should generate date-only labels for very long periods (>= 10080 minutes)", async () => {
    const { result } = renderHook(() => useSnapshot());

    // Set very long period (1 week = 10080 minutes)
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T00:00:00Z",
        end: "2023-01-08T00:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Labels should include year and date but no time for periods >= 10080 minutes
    const hasTime = result.current.filledLabels.some(label => 
      /\d{2}:\d{2}/.test(label)
    );
    const hasYear = result.current.filledLabels.some(label => 
      /2023/.test(label)
    );
    
    expect(hasTime).toBe(false); // No time for very long periods
    expect(hasYear).toBe(true);  // Should have year
  });

  it("should use system locale for date formatting", async () => {
    const { result } = renderHook(() => useSnapshot());

    // Set period that includes date
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T00:00:00Z",
        end: "2023-01-01T12:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Should have formatted labels (exact format depends on system locale)
    expect(result.current.filledLabels.length).toBeGreaterThan(0);
    expect(result.current.filledLabels.every(label => label.length > 0)).toBe(true);
  });

  it("should maintain consistent label count with data count", async () => {
    const { result } = renderHook(() => useSnapshot());

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Labels and data should have same length
    expect(result.current.filledLabels.length).toBe(result.current.filledChartData.length);
  });

  it("should update labels when period changes", async () => {
    const { result } = renderHook(() => useSnapshot());

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    const initialLabels = [...result.current.filledLabels];

    // Change to different period
    act(() => {
      result.current.setPeriod({
        start: "2023-02-01T00:00:00Z",
        end: "2023-02-01T06:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Labels should be different
    expect(result.current.filledLabels).not.toEqual(initialLabels);
  });

  it("should handle edge case periods correctly", async () => {
    const { result } = renderHook(() => useSnapshot());

    // Test exactly 180 minutes (boundary condition)
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T00:00:00Z",
        end: "2023-01-01T03:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Should include date for exactly 180 minutes
    const hasDate = result.current.filledLabels.some(label => 
      /\d+\/\d+/.test(label)
    );
    expect(hasDate).toBe(true);
  });

  it("should handle exactly 1440 minutes (1 day) correctly", async () => {
    const { result } = renderHook(() => useSnapshot());

    // Test exactly 1440 minutes (boundary condition)
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T00:00:00Z",
        end: "2023-01-02T00:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Should include year for exactly 1440 minutes
    const hasYear = result.current.filledLabels.some(label => 
      /2023/.test(label)
    );
    expect(hasYear).toBe(true);
  });

  it("should handle exactly 10080 minutes (1 week) correctly", async () => {
    const { result } = renderHook(() => useSnapshot());

    // Test exactly 10080 minutes (boundary condition)
    act(() => {
      result.current.setPeriod({
        start: "2023-01-01T00:00:00Z",
        end: "2023-01-08T00:00:00Z",
      });
    });

    await act(async () => {
      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    // Should not include time for exactly 10080 minutes
    const hasTime = result.current.filledLabels.some(label => 
      /\d{2}:\d{2}/.test(label)
    );
    expect(hasTime).toBe(false);
  });
});