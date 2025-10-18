// src/test/unit/useColorTheme.test.ts
import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useColorTheme } from "@/hooks/useColorTheme";
import type { Theme } from "@/rspc/bindings";

// Create mock functions that can be reconfigured per test
const mockTheme = vi.fn();
const mockOnThemeChanged = vi.fn();

// Mock Tauri API
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    theme: mockTheme,
    onThemeChanged: mockOnThemeChanged,
  }),
}));

describe("useColorTheme", () => {
  // 各テスト実行前に、document のクラスリストをリセット
  beforeEach(() => {
    document.documentElement.classList.remove("dark", "light");
    document.documentElement.dataset.theme = "";
    vi.clearAllMocks();

    // デフォルトのモック動作を設定
    mockTheme.mockResolvedValue("light");
    mockOnThemeChanged.mockResolvedValue(() => {});
  });

  it("デフォルトでは 'light' で初期化される", () => {
    renderHook(() => useColorTheme("light"));
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("初期値として 'dark' が渡された場合、正しく初期化される", () => {
    renderHook(() => useColorTheme("dark"));
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("状態が指定されたテーマに変更されること", () => {
    const { rerender } = renderHook(({ theme }) => useColorTheme(theme), {
      initialProps: { theme: "light" as Theme },
    });
    expect(document.documentElement.classList.contains("dark")).toBe(false);

    rerender({ theme: "dark" as Theme });
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    rerender({ theme: "light" as Theme });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("darkClasses に含まれるテーマが 'dark' クラスを追加する", () => {
    const darkThemes: Theme[] = ["dark", "nebula", "espresso"];

    darkThemes.forEach((theme) => {
      renderHook(() => useColorTheme(theme));
      expect(document.documentElement.classList.contains("dark")).toBe(true);
      document.documentElement.classList.remove("dark"); // 次のループのためにリセット
    });
  });

  it("darkClasses に含まれないテーマが 'dark' クラスを追加しない", () => {
    const nonDarkThemes: Theme[] = [
      "light",
      "ocean",
      "grove",
      "sunset",
      "orbit",
      "cappuccino",
    ];

    nonDarkThemes.forEach((theme) => {
      renderHook(() => useColorTheme(theme));
      expect(document.documentElement.classList.contains("dark")).toBe(false);
    });
  });

  it("'system' テーマでシステムテーマが 'light' の場合、'light' クラスが追加される", async () => {
    renderHook(() => useColorTheme("system"));

    await waitFor(() => {
      expect(document.documentElement.classList.contains("light")).toBe(true);
    });
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("'system' テーマでシステムテーマが 'dark' の場合、'dark' クラスが追加される", async () => {
    // このテスト用にdarkテーマを返すようにモックを設定
    mockTheme.mockResolvedValue("dark");

    renderHook(() => useColorTheme("system"));

    await waitFor(() => {
      expect(document.documentElement.classList.contains("dark")).toBe(true);
    });
    expect(document.documentElement.classList.contains("light")).toBe(false);
  });
});
