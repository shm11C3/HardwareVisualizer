import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

interface FakeStore {
  data: Record<string, unknown>;
  has: (key: string) => Promise<boolean>;
  get: <T = unknown>(key: string) => Promise<T | undefined>;
  set: <T>(key: string, value: T) => Promise<void>;
  save: () => Promise<void>;
}

// This variable will hold a new store object for each test
let fakeStore: FakeStore;

// Variable for reloading the useTauriStore hook
let useTauriStore: <T>(
  key: string,
  defaultValue: T,
) => [T | null, (newValue: T) => Promise<void>, boolean];

describe("useTauriStore", () => {
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
    // Return fakeStore as the return value of storePromise (load("store.json", { autoSave: true }))
    vi.doMock("@tauri-apps/plugin-store", () => ({
      load: vi.fn(() => Promise.resolve(fakeStore)),
    }));

    // Reload the module containing useTauriStore
    const module = await import("@/hooks/useTauriStore");
    useTauriStore = <T>(key: string, defaultValue: T) => {
      return module.useTauriStore<T>(key, defaultValue);
    };
  });

  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("When key exists on initial load, value from store is returned", async () => {
    // Arrange: State where value is already saved in fakeStore
    fakeStore.data.testKey = "storedValue";
    const { result } = renderHook(() =>
      useTauriStore<string>("testKey", "defaultValue"),
    );

    // React 19.1.0 compatible: Wrap with act to ensure state update completes
    await act(async () => {
      await waitFor(
        () => !result.current[2] && result.current[0] === "storedValue",
      );
    });

    expect(result.current[0]).toBe("storedValue");
    expect(fakeStore.has).toHaveBeenCalledWith("testKey");
    expect(fakeStore.get).toHaveBeenCalledWith("testKey");
  });

  it("When key does not exist on initial load, default value is set and returned", async () => {
    // Arrange: State where "nonExisting" does not exist in fakeStore.data
    const { result } = renderHook(() =>
      useTauriStore<string>("nonExisting", "defaultValue"),
    );

    // React 19.1.0 compatible: Wrap with act to ensure state update completes
    await act(async () => {
      await waitFor(
        () => !result.current[2] && result.current[0] === "defaultValue",
      );
    });

    // Since key did not exist, defaultValue is set
    expect(result.current[0]).toBe("defaultValue");
    // Since it did not exist, set and save are called
    expect(fakeStore.set).toHaveBeenCalledWith("nonExisting", "defaultValue");
    expect(fakeStore.save).toHaveBeenCalled();
  });

  it("When setValue is called, value is updated", async () => {
    const { result } = renderHook(() =>
      useTauriStore<string>("testKey", "defaultValue"),
    );

    // React 19.1.0 compatible: Wrap with act to ensure initial state update completes
    await act(async () => {
      await waitFor(
        () => !result.current[2] && result.current[0] === "defaultValue",
      );
    });
    expect(result.current[0]).toBe("defaultValue");

    // Act: Call setValue to update value
    await act(async () => {
      await result.current[1]("newValue");
    });

    // Assert: state is updated to new value
    expect(result.current[0]).toBe("newValue");
    expect(fakeStore.set).toHaveBeenCalledWith("testKey", "newValue");
    expect(fakeStore.save).toHaveBeenCalled();
  });

  it("Can handle undefined defaultValue", async () => {
    const { result } = renderHook(() =>
      useTauriStore<undefined>("testKey", undefined),
    );

    // React 19.1.0 compatible: Wrap with act to ensure state update completes
    await act(async () => {
      await waitFor(() => !result.current[2]);
    });

    expect(result.current[0]).toBeUndefined();
  });

  it("isPending is true while loading", async () => {
    const { result } = renderHook(() => useTauriStore("someKey", "someValue"));
    expect(result.current[2]).toBe(true);
    await waitFor(() => result.current[2] === false);
  });
});
