import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Will be assigned per test before importing the module under test
let mockLoad: ReturnType<
  typeof vi.fn<
    (...args: unknown[]) => Promise<{
      select: ReturnType<typeof vi.fn>;
      execute: ReturnType<typeof vi.fn>;
    }>
  >
>;

// Mock the Tauri SQL plugin so we can control DB behaviors
vi.mock("@tauri-apps/plugin-sql", () => {
  return {
    default: {
      // Delegate to a per-test mock so we can vary behavior
      load: (...args: unknown[]) => mockLoad(...args),
    },
  };
});

let errorSpy: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  vi.resetModules();
  errorSpy = vi.spyOn(console, "error").mockImplementation(() => {});
});

afterEach(() => {
  errorSpy.mockRestore();
});

describe("sqlite wrapper", () => {
  it("load returns rows on success", async () => {
    const rows = [{ id: 1 }];
    const select = vi.fn().mockResolvedValue(rows);
    const execute = vi.fn().mockResolvedValue(undefined);
    mockLoad = vi.fn().mockResolvedValue({ select, execute });

    const { sqlitePromise } = await import("@/lib/sqlite");
    const db = await sqlitePromise;

    const res = await db.load<(typeof rows)[number]>("SELECT 1");
    expect(res).toEqual(rows);
    expect(select).toHaveBeenCalledWith("SELECT 1");
  });

  it("load returns [] and logs on error", async () => {
    const select = vi.fn().mockRejectedValue(new Error("boom"));
    const execute = vi.fn().mockResolvedValue(undefined);
    mockLoad = vi.fn().mockResolvedValue({ select, execute });

    const { sqlitePromise } = await import("@/lib/sqlite");
    const db = await sqlitePromise;

    const res = await db.load<unknown>("SELECT broken");
    expect(res).toEqual([]);
    expect(errorSpy).toHaveBeenCalled();
  });

  it("save calls execute on success", async () => {
    const select = vi.fn();
    const execute = vi.fn().mockResolvedValue(undefined);
    mockLoad = vi.fn().mockResolvedValue({ select, execute });

    const { sqlitePromise } = await import("@/lib/sqlite");
    const db = await sqlitePromise;

    await db.save("INSERT ...");
    expect(execute).toHaveBeenCalledWith("INSERT ...");
  });

  it("save swallows errors and logs", async () => {
    const select = vi.fn();
    const execute = vi.fn().mockRejectedValue(new Error("write-fail"));
    mockLoad = vi.fn().mockResolvedValue({ select, execute });

    const { sqlitePromise } = await import("@/lib/sqlite");
    const db = await sqlitePromise;

    await expect(db.save("INSERT ...")).resolves.toBeUndefined();
    expect(errorSpy).toHaveBeenCalled();
  });
});
