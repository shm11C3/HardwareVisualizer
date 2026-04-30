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
let useCloseToTrayPreference: typeof import("./useCloseToTrayPreference").useCloseToTrayPreference;
let setCloseToTrayPreference: typeof import("./useCloseToTrayPreference").setCloseToTrayPreference;

const setupModule = async () => {
  const module = await import("./useCloseToTrayPreference");
  useCloseToTrayPreference = module.useCloseToTrayPreference;
  setCloseToTrayPreference = module.setCloseToTrayPreference;
};

describe("useCloseToTrayPreference", () => {
  beforeEach(async () => {
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

    await setupModule();
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.resetModules();
  });

  it("loads unset preference as disabled with no choice made", async () => {
    const { result } = renderHook(() => useCloseToTrayPreference());

    await waitFor(() => expect(result.current.isPending).toBe(false));

    expect(result.current.closeToTray).toBe(false);
    expect(result.current.choiceMade).toBe(false);
    expect(fakeStore.set).not.toHaveBeenCalled();
  });

  it("loads persisted close-to-tray preference", async () => {
    fakeStore.data.closeToTray = true;
    fakeStore.data.closeToTrayChoiceMade = true;

    const { result } = renderHook(() => useCloseToTrayPreference());

    await waitFor(() => expect(result.current.isPending).toBe(false));

    expect(result.current.closeToTray).toBe(true);
    expect(result.current.choiceMade).toBe(true);
    expect(fakeStore.get).toHaveBeenCalledWith("closeToTray");
    expect(fakeStore.get).toHaveBeenCalledWith("closeToTrayChoiceMade");
  });

  it("persists both preference keys from the direct setter", async () => {
    await setCloseToTrayPreference(true);

    expect(fakeStore.set).toHaveBeenCalledWith("closeToTray", true);
    expect(fakeStore.set).toHaveBeenCalledWith("closeToTrayChoiceMade", true);
    expect(fakeStore.save).toHaveBeenCalledOnce();
  });

  it("updates hook state only after persistence succeeds", async () => {
    const { result } = renderHook(() => useCloseToTrayPreference());

    await waitFor(() => expect(result.current.isPending).toBe(false));

    await act(async () => {
      const saved = await result.current.setCloseToTray(true);
      expect(saved).toBe(true);
    });

    expect(result.current.closeToTray).toBe(true);
    expect(result.current.choiceMade).toBe(true);
    expect(fakeStore.set).toHaveBeenCalledWith("closeToTray", true);
    expect(fakeStore.set).toHaveBeenCalledWith("closeToTrayChoiceMade", true);
  });

  it("clears pending state when preference loading fails", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    vi.mocked(fakeStore.has).mockRejectedValueOnce(new Error("store failed"));

    const { result } = renderHook(() => useCloseToTrayPreference());

    await waitFor(() => expect(result.current.isPending).toBe(false));

    expect(result.current.closeToTray).toBe(false);
    expect(result.current.choiceMade).toBe(false);
    expect(errorSpy).toHaveBeenCalledWith(
      "Failed to load close-to-tray preference:",
      expect.any(Error),
    );
  });

  it("returns false and keeps state unchanged when saving fails", async () => {
    const errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
    const { result } = renderHook(() => useCloseToTrayPreference());

    await waitFor(() => expect(result.current.isPending).toBe(false));
    vi.mocked(fakeStore.set).mockRejectedValueOnce(new Error("save failed"));

    await act(async () => {
      const saved = await result.current.setCloseToTray(true);
      expect(saved).toBe(false);
    });

    expect(result.current.closeToTray).toBe(false);
    expect(result.current.choiceMade).toBe(false);
    expect(errorSpy).toHaveBeenCalledWith(
      "Failed to save close-to-tray preference:",
      expect.any(Error),
    );
  });
});
