import { act, renderHook, waitFor } from "@testing-library/react";
import { Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  type DashboardSelectItemType,
  dashBoardItems,
} from "@/features/hardware/dashboard/types/dashboardItem";

interface FakeStore {
  data: Record<string, unknown>;
  has: (key: string) => Promise<boolean>;
  get: <T = unknown>(key: string) => Promise<T | undefined>;
  set: <T>(key: string, value: T) => Promise<void>;
  save: () => Promise<void>;
}

// This variable will hold a new store object for each test
let fakeStore: FakeStore;

// Mock toggleTitleIconVisibility function
const mockToggleTitleIconVisibility = vi.fn();

// Variable for reloading the useDashboardSelector hook
let useDashboardSelector: () => {
  visibleItems: DashboardSelectItemType[] | null;
  toggleItem: (item: DashboardSelectItemType) => Promise<void>;
};

describe("useDashboardSelector", () => {
  beforeEach(async () => {
    // Clear module cache to enable reloading
    vi.resetModules();

    // Create new fakeStore for each test
    fakeStore = {
      data: {},
      has: vi.fn((key: string) => Promise.resolve(key in fakeStore.data)),
      get: vi
        .fn()
        .mockImplementation(<T>(key: string) =>
          Promise.resolve(fakeStore.data[key] as T | undefined),
        ) as <T = unknown>(key: string) => Promise<T | undefined>,
      set: vi.fn(<T>(key: string, value: T) => {
        fakeStore.data[key] = value;
        return Promise.resolve();
      }),
      save: vi.fn(() => Promise.resolve()),
    };

    // Mock @tauri-apps/plugin-store module
    vi.doMock("@tauri-apps/plugin-store", () => ({
      load: vi.fn(() => Promise.resolve(fakeStore)),
    }));

    // Mock useTitleIconVisualSelector
    vi.doMock("@/hooks/useTitleIconVisualSelector", () => ({
      useTitleIconVisualSelector: () => ({
        toggleTitleIconVisibility: mockToggleTitleIconVisibility,
      }),
    }));

    // Reload the module containing useDashboardSelector
    const module = await import(
      "@/features/hardware/dashboard/hooks/useDashboardSelector"
    );
    useDashboardSelector = module.useDashboardSelector;
  });

  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("initializes with all dashboard items plus 'title' when no stored value exists", async () => {
    const { result } = renderHook(() => useDashboardSelector(), {
      wrapper: Provider,
    });

    // Wait for the hook to finish loading
    await act(async () => {
      await waitFor(() => result.current.visibleItems !== null);
    });

    // Verify that visibleItems contains all dashboard items plus 'title'
    const expectedItems = [...dashBoardItems, "title"];
    expect(result.current.visibleItems).toEqual(
      expect.arrayContaining(expectedItems),
    );
    expect(result.current.visibleItems?.length).toBe(expectedItems.length);

    // Verify that the default value was set in the store
    expect(fakeStore.set).toHaveBeenCalledWith(
      "dashboardVisibleItems",
      expect.arrayContaining(expectedItems),
    );
  });

  it("loads stored value when it exists in the store", async () => {
    // Arrange: Store already has a value
    const storedItems: DashboardSelectItemType[] = ["cpu", "memory", "title"];
    fakeStore.data.dashboardVisibleItems = storedItems;

    const { result } = renderHook(() => useDashboardSelector(), {
      wrapper: Provider,
    });

    // Wait for the hook to finish loading
    await act(async () => {
      await waitFor(() => result.current.visibleItems !== null);
    });

    // Verify that visibleItems matches the stored value
    expect(result.current.visibleItems).toEqual(storedItems);
  });

  it("toggles visibility of an item by adding it when not present", async () => {
    // Arrange: Start with only some items visible
    const initialItems: DashboardSelectItemType[] = ["cpu", "memory"];
    fakeStore.data.dashboardVisibleItems = initialItems;

    const { result } = renderHook(() => useDashboardSelector(), {
      wrapper: Provider,
    });

    // Wait for the hook to finish loading
    await act(async () => {
      await waitFor(() => result.current.visibleItems !== null);
    });

    // Act: Toggle 'gpu' to add it
    await act(async () => {
      await result.current.toggleItem("gpu");
    });

    // Assert: 'gpu' is now in the list
    expect(result.current.visibleItems).toContain("gpu");
    expect(result.current.visibleItems).toEqual(
      expect.arrayContaining(["cpu", "memory", "gpu"]),
    );
  });

  it("toggles visibility of an item by removing it when present", async () => {
    // Arrange: Start with multiple items visible
    const initialItems: DashboardSelectItemType[] = ["cpu", "memory", "gpu"];
    fakeStore.data.dashboardVisibleItems = initialItems;

    const { result } = renderHook(() => useDashboardSelector(), {
      wrapper: Provider,
    });

    // Wait for the hook to finish loading
    await act(async () => {
      await waitFor(() => result.current.visibleItems !== null);
    });

    // Act: Toggle 'gpu' to remove it
    await act(async () => {
      await result.current.toggleItem("gpu");
    });

    // Assert: 'gpu' is no longer in the list
    expect(result.current.visibleItems).not.toContain("gpu");
    expect(result.current.visibleItems).toEqual(["cpu", "memory"]);
  });

  it("prevents empty selection when trying to remove the last item", async () => {
    // Arrange: Start with only one item visible
    const initialItems: DashboardSelectItemType[] = ["cpu"];
    fakeStore.data.dashboardVisibleItems = initialItems;

    const { result } = renderHook(() => useDashboardSelector(), {
      wrapper: Provider,
    });

    // Wait for the hook to finish loading
    await act(async () => {
      await waitFor(() => result.current.visibleItems !== null);
    });

    // Act: Try to toggle 'cpu' to remove it (the last item)
    await act(async () => {
      await result.current.toggleItem("cpu");
    });

    // Assert: 'cpu' is still in the list (empty selection was prevented)
    expect(result.current.visibleItems).toEqual(["cpu"]);
  });

  it("calls toggleTitleIconVisibility with true when title is in visibleItems", async () => {
    // Arrange: Start with 'title' included
    const initialItems: DashboardSelectItemType[] = ["cpu", "title"];
    fakeStore.data.dashboardVisibleItems = initialItems;

    const { result } = renderHook(() => useDashboardSelector(), {
      wrapper: Provider,
    });

    // Wait for the hook to finish loading
    await act(async () => {
      await waitFor(() => result.current.visibleItems !== null);
    });

    // Verify toggleTitleIconVisibility was called with the correct arguments
    expect(mockToggleTitleIconVisibility).toHaveBeenCalledWith(
      "dashboard",
      true,
    );
  });

  it("calls toggleTitleIconVisibility with false when title is not in visibleItems", async () => {
    // Arrange: Start without 'title'
    const initialItems: DashboardSelectItemType[] = ["cpu", "memory"];
    fakeStore.data.dashboardVisibleItems = initialItems;

    const { result } = renderHook(() => useDashboardSelector(), {
      wrapper: Provider,
    });

    // Wait for the hook to finish loading
    await act(async () => {
      await waitFor(() => result.current.visibleItems !== null);
    });

    // Verify toggleTitleIconVisibility was called with false
    expect(mockToggleTitleIconVisibility).toHaveBeenCalledWith(
      "dashboard",
      false,
    );
  });
});
