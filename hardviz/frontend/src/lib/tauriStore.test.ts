import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { clearTauriStore, getStoreInstance } from "@/lib/tauriStore";

// Mock @tauri-apps/plugin-store
vi.mock("@tauri-apps/plugin-store", () => ({
  load: vi.fn(),
}));

describe("tauriStore", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("should create and return store instance", async () => {
    const mockStore = {
      get: vi.fn(),
      set: vi.fn(),
      save: vi.fn(),
      reset: vi.fn(),
    };
    const { load } = await vi.importMock("@tauri-apps/plugin-store");
    (load as Mock).mockResolvedValueOnce(mockStore);

    const store = await getStoreInstance();

    expect(load).toHaveBeenCalledWith("store.json", {
      autoSave: true,
      defaults: {},
    });
    expect(store).toHaveProperty("get");
    expect(store).toHaveProperty("set");
    expect(store).toHaveProperty("save");
  });

  it("should return same instance on subsequent calls", async () => {
    const store1 = await getStoreInstance();
    const store2 = await getStoreInstance();

    expect(store1).toBe(store2);
  });

  it("should handle store loading correctly", async () => {
    const store = await getStoreInstance();

    expect(store).toHaveProperty("get");
    expect(store).toHaveProperty("set");
    expect(store).toHaveProperty("save");
  });

  it("should call store.reset when clearTauriStore is called", async () => {
    const store = await getStoreInstance();
    const resetSpy = vi.spyOn(store, "reset").mockResolvedValueOnce(undefined);

    await clearTauriStore();

    expect(resetSpy).toHaveBeenCalledOnce();
  });
});
