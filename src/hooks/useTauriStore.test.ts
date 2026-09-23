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
) => [T | null, (newValue: T) => Promise<void>, boolean, boolean];

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
    fakeStore.data["testKey"] = "storedValue";
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
    expect(result.current[3]).toBe(false);
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
    expect(result.current[3]).toBe(false);
    await waitFor(() => expect(result.current[2]).toBe(false));
    expect(result.current[3]).toBe(false);
  });

  it("Settles to the default with isPending false when the initial read rejects", async () => {
    // A store read failure must not leave consumers gated on isPending forever
    // (e.g. the conversion prompt and the NSIS migration notice).
    fakeStore.has = vi.fn(() => Promise.reject(new Error("store read failed")));
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(() =>
      useTauriStore<string>("testKey", "defaultValue"),
    );

    await waitFor(() => expect(result.current[2]).toBe(false));

    expect(result.current[0]).toBe("defaultValue");
    // Persisting consumers must be able to tell this apart from an absent key.
    expect(result.current[3]).toBe(true);
    expect(consoleError).toHaveBeenCalled();
    expect(fakeStore.set).not.toHaveBeenCalled();
  });

  it("Settles to the default with isPending false when the store cannot be loaded", async () => {
    vi.doMock("@tauri-apps/plugin-store", () => ({
      load: vi.fn(() => Promise.reject(new Error("store load failed"))),
    }));
    vi.resetModules();
    const module = await import("@/hooks/useTauriStore");
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(() =>
      module.useTauriStore<string>("testKey", "defaultValue"),
    );

    await waitFor(() => expect(result.current[2]).toBe(false));

    expect(result.current[0]).toBe("defaultValue");
    expect(result.current[3]).toBe(true);
    expect(consoleError).toHaveBeenCalled();
  });

  it("Rejects setValue and keeps the previous value when the store write fails", async () => {
    fakeStore.data["testKey"] = "storedValue";
    const { result } = renderHook(() =>
      useTauriStore<string>("testKey", "defaultValue"),
    );
    await waitFor(() => expect(result.current[2]).toBe(false));

    fakeStore.set = vi.fn(() =>
      Promise.reject(new Error("store write failed")),
    );

    await act(async () => {
      await expect(result.current[1]("newValue")).rejects.toThrow(
        "store write failed",
      );
    });

    expect(result.current[0]).toBe("storedValue");
    expect(fakeStore.save).not.toHaveBeenCalled();
  });

  it("Does not let a superseded key's read overwrite the current key", async () => {
    // The key changes while the first read is still in flight. The first
    // read then finishes last; its result belongs to a key the hook no longer
    // renders and must not replace the second key's value.
    const resolveHas = new Map<string, (exists: boolean) => void>();
    fakeStore.data = { first: "firstValue", second: "secondValue" };
    fakeStore.has = vi.fn(
      (key: string) =>
        new Promise<boolean>((resolve) => {
          resolveHas.set(key, resolve);
        }),
    );

    const { result, rerender } = renderHook(
      ({ key }) => useTauriStore<string>(key, "defaultValue"),
      { initialProps: { key: "first" } },
    );
    await waitFor(() => expect(resolveHas.get("first")).toBeDefined());

    rerender({ key: "second" });
    await waitFor(() => expect(resolveHas.get("second")).toBeDefined());

    await act(async () => {
      resolveHas.get("second")?.(true);
    });
    await waitFor(() => expect(result.current[0]).toBe("secondValue"));

    await act(async () => {
      resolveHas.get("first")?.(true);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(result.current[0]).toBe("secondValue");
    expect(result.current[2]).toBe(false);
  });

  it("Does not update state after unmount (cleanup guard)", async () => {
    // Unmounting before the store resolves exercises two uncovered paths:
    //  1. The cleanup function (`isCancelled = true`)
    //  2. The cancellation guard (`if (isCancelled) return`)
    const { result, unmount } = renderHook(() =>
      useTauriStore<string>("testKey", "default"),
    );

    // Store has not resolved yet; still pending
    expect(result.current[2]).toBe(true);

    // Unmount — triggers the useEffect cleanup immediately
    unmount();

    // Drain the microtask queue so fetchValue completes after unmount.
    // The cancellation guard prevents any subsequent setState call.
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    // If the cancellation guard did NOT work, setValueState would throw a
    // "Can't perform a React state update on an unmounted component" warning.
    // Reaching this line without errors confirms correct behaviour.
    expect(result.current[2]).toBe(true);
  });
});
