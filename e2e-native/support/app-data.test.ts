import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import {
  assertSafeE2eDataDir,
  E2E_APP_IDENTIFIER,
  resolveE2eAppDataDir,
} from "./app-data";

describe("native E2E app data paths", () => {
  it("accepts a directory whose basename is the E2E app identifier", () => {
    const safePath = path.join(os.tmpdir(), "hardviz-test", E2E_APP_IDENTIFIER);

    expect(assertSafeE2eDataDir(safePath)).toBe(path.resolve(safePath));
  });

  it("rejects release and development app data directories", () => {
    expect(() =>
      assertSafeE2eDataDir(path.join(os.tmpdir(), "HardwareVisualizer")),
    ).toThrow(/non-E2E/);
    expect(() =>
      assertSafeE2eDataDir(path.join(os.tmpdir(), "HardwareVisualizerDev")),
    ).toThrow(/non-E2E/);
  });

  it("rejects child paths inside the E2E app data directory", () => {
    expect(() =>
      assertSafeE2eDataDir(
        path.join(os.tmpdir(), E2E_APP_IDENTIFIER, "settings.json"),
      ),
    ).toThrow(/non-E2E/);
  });

  it("rejects root-like paths even when the basename matches", () => {
    const rootChild = path.join(
      path.parse(process.cwd()).root,
      E2E_APP_IDENTIFIER,
    );

    expect(() => assertSafeE2eDataDir(rootChild)).toThrow(/root-like/);
  });

  it("resolves Windows app data under APPDATA", () => {
    const result = resolveE2eAppDataDir("win32", {
      APPDATA: "C:\\Users\\Test\\AppData\\Roaming",
    });

    expect(path.basename(result)).toBe(E2E_APP_IDENTIFIER);
    expect(result).toContain("AppData");
  });

  it("resolves Linux app data under HOME/.config", () => {
    const result = resolveE2eAppDataDir("linux", {
      HOME: path.join(os.tmpdir(), "test-home"),
    });

    expect(result).toBe(
      path.resolve(os.tmpdir(), "test-home", ".config", E2E_APP_IDENTIFIER),
    );
  });

  it("resolves macOS app data under Application Support", () => {
    const result = resolveE2eAppDataDir("darwin", {
      HOME: path.join(os.tmpdir(), "test-home"),
    });

    expect(result).toBe(
      path.resolve(
        os.tmpdir(),
        "test-home",
        "Library",
        "Application Support",
        E2E_APP_IDENTIFIER,
      ),
    );
  });

  it("requires platform data directory environment variables", () => {
    expect(() => resolveE2eAppDataDir("win32", {})).toThrow(/APPDATA/);
    expect(() => resolveE2eAppDataDir("linux", {})).toThrow(/HOME/);
    expect(() => resolveE2eAppDataDir("darwin", {})).toThrow(/HOME/);
  });
});
