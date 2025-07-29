// src/test/unit/useColorTheme.test.ts
import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { useColorTheme } from "@/hooks/useColorTheme";
import type { Theme } from "@/rspc/bindings";

describe("useColorTheme", () => {
  // 各テスト実行前に、document のクラスリストをリセット
  beforeEach(() => {
    document.documentElement.classList.remove("dark");
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
});
