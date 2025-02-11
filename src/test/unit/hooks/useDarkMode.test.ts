import { useDarkMode } from "@/hooks/useDarkMode";
// src/test/unit/useDarkMode.test.ts
import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";

describe("useDarkMode", () => {
  // 各テスト実行前に、document のクラスリストをリセット
  beforeEach(() => {
    document.documentElement.classList.remove("dark");
  });

  it("デフォルトでは false で初期化される", () => {
    const { result } = renderHook(() => useDarkMode());
    expect(result.current.isDarkMode).toBe(false);
    // 初期状態なので document に "dark" クラスが付与されていないことを確認
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("初期値として渡された場合、true で初期化される", () => {
    const { result } = renderHook(() => useDarkMode(true));
    expect(result.current.isDarkMode).toBe(true);
    // 初期値が true なので、document に "dark" クラスが付与されていることを確認
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("引数なしでtoggleが呼び出されたとき、状態を切り替える", () => {
    const { result } = renderHook(() => useDarkMode());
    // 初期値は false
    expect(result.current.isDarkMode).toBe(false);

    act(() => {
      result.current.toggle();
    });
    // toggle() 呼び出しで状態が反転し true になる
    expect(result.current.isDarkMode).toBe(true);
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    act(() => {
      result.current.toggle();
    });
    // 再度 toggle() 呼び出しで false へ戻る
    expect(result.current.isDarkMode).toBe(false);
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });

  it("状態が指定された値に変更されること", () => {
    const { result } = renderHook(() => useDarkMode());
    act(() => {
      result.current.toggle(true);
    });
    expect(result.current.isDarkMode).toBe(true);
    expect(document.documentElement.classList.contains("dark")).toBe(true);

    act(() => {
      result.current.toggle(false);
    });
    expect(result.current.isDarkMode).toBe(false);
    expect(document.documentElement.classList.contains("dark")).toBe(false);
  });
});
