import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

interface FakeStore {
  data: Record<string, unknown>;
  has: (key: string) => Promise<boolean>;
  get: <T = unknown>(key: string) => Promise<T | undefined>;
  set: <T>(key: string, value: T) => Promise<void>;
  save: () => Promise<void>;
}

let fakeStore: FakeStore;
let useDatabaseConversionNoticeShown: () => [
  boolean,
  (value: boolean) => Promise<void>,
  boolean,
];

const STORE_KEY = "databaseConversionCompleteNoticeShown";

describe("useDatabaseConversionNoticeShown", () => {
  beforeEach(async () => {
    // Reload the module (and its module-level Jotai atom) fresh for each
    // test, the same isolation pattern useTauriStore's own test uses -
    // otherwise the atom's in-memory value would leak between tests.
    vi.resetModules();

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

    vi.doMock("@tauri-apps/plugin-store", () => ({
      load: vi.fn(() => Promise.resolve(fakeStore)),
    }));

    const module = await import("./useDatabaseConversionNoticeShown");
    useDatabaseConversionNoticeShown = module.useDatabaseConversionNoticeShown;
  });

  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("defaults to false when nothing was ever saved", async () => {
    const { result } = renderHook(() => useDatabaseConversionNoticeShown());

    expect(result.current[2]).toBe(true);
    await waitFor(() => expect(result.current[2]).toBe(false));
    expect(result.current[0]).toBe(false);
    expect(fakeStore.set).toHaveBeenCalledWith(STORE_KEY, false);
  });

  it("loads an already-saved value", async () => {
    fakeStore.data[STORE_KEY] = true;
    const { result } = renderHook(() => useDatabaseConversionNoticeShown());

    await waitFor(() => expect(result.current[2]).toBe(false));
    expect(result.current[0]).toBe(true);
  });

  it("setShown updates the value and persists it", async () => {
    const { result } = renderHook(() => useDatabaseConversionNoticeShown());
    await waitFor(() => expect(result.current[2]).toBe(false));

    await act(async () => {
      await result.current[1](true);
    });

    expect(result.current[0]).toBe(true);
    expect(fakeStore.set).toHaveBeenCalledWith(STORE_KEY, true);
    expect(fakeStore.save).toHaveBeenCalled();
  });

  it("shares state across every mounted consumer (Settings section + the app-root prompt dialog)", async () => {
    const first = renderHook(() => useDatabaseConversionNoticeShown());
    const second = renderHook(() => useDatabaseConversionNoticeShown());

    await waitFor(() => expect(first.result.current[2]).toBe(false));
    await waitFor(() => expect(second.result.current[2]).toBe(false));
    expect(first.result.current[0]).toBe(false);
    expect(second.result.current[0]).toBe(false);

    // Dismissing through the first mount must be immediately visible to
    // the second, without the second needing to reload from the store -
    // otherwise both could independently decide the notice still needs
    // showing.
    await act(async () => {
      await first.result.current[1](true);
    });

    expect(first.result.current[0]).toBe(true);
    expect(second.result.current[0]).toBe(true);
  });
});
