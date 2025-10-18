import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
// src/test/unit/useSettingsAtom.test.ts
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

/**
 * モックの設定
 */
const errorMock = vi.fn();

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: errorMock,
  }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getSettings: vi.fn(),
    setTheme: vi.fn(),
    setDisplayTargets: vi.fn(),
    setGraphSize: vi.fn(),
    setLineGraphType: vi.fn(),
    setLanguage: vi.fn(),
    setLineGraphBorder: vi.fn(),
    setLineGraphFill: vi.fn(),
    setLineGraphMix: vi.fn(),
    setLineGraphShowLegend: vi.fn(),
    setLineGraphShowScale: vi.fn(),
    setLineGraphShowTooltip: vi.fn(),
    setBackgroundImgOpacity: vi.fn(),
    setSelectedBackgroundImg: vi.fn(),
    setTemperatureUnit: vi.fn(),
    setLineGraphColor: vi.fn(),
  },
}));

/**
 * テスト対象のフックをインポート
 */
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { commands } from "@/rspc/bindings";

/**
 * テスト実行
 */
describe("useSettingsAtom", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loadSettings: 成功時に settings を更新する", async () => {
    // テスト用の設定データ
    const settingsData = {
      version: "1.0.0",
      language: "ja",
      theme: "dark",
      displayTargets: ["cpu"],
      graphSize: "lg",
      lineGraphType: "custom",
      lineGraphBorder: false,
      lineGraphFill: false,
      lineGraphColor: {
        cpu: "rgb(255,0,0)",
        memory: "rgb(0,255,0)",
        gpu: "rgb(0,0,255)",
      },
      lineGraphMix: false,
      lineGraphShowLegend: false,
      lineGraphShowScale: true,
      lineGraphShowTooltip: false,
      backgroundImgOpacity: 70,
      selectedBackgroundImg: "image.png",
      temperatureUnit: "F",
    };

    // commands.getSettings が成功結果を返すようにモック
    (commands.getSettings as Mock).mockResolvedValue({ data: settingsData });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.loadSettings();
    });
    expect(result.current.settings).toEqual(settingsData);
  });

  it("loadSettings: エラー時に error() が呼ばれ、settings が初期値のまま", async () => {
    const errorMsg = "Failed to fetch settings";
    (commands.getSettings as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    // loadSettings 前の初期状態を保存
    const initialSettings = result.current.settings;
    await act(async () => {
      await result.current.loadSettings();
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings).toEqual(initialSettings);
  });

  it("updateSettingAtom: 成功時に設定が更新される", async () => {
    // 例として "theme" の更新テスト
    (commands.setTheme as Mock).mockResolvedValue({ data: null });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    // 初期状態では theme は "light"（settingsAtom のデフォルト値）
    await act(async () => {
      await result.current.updateSettingAtom("theme", "dark");
    });
    expect(result.current.settings.theme).toEqual("dark");
  });

  it("updateSettingAtom: エラー時に error() が呼ばれ、値が元に戻される", async () => {
    const errorMsg = "Failed to update theme";
    (commands.setTheme as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    // 初期状態の theme は "system"
    await act(async () => {
      await result.current.updateSettingAtom("theme", "dark");
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    // 更新失敗時は元の値 ("system") に戻る
    expect(result.current.settings.theme).toEqual("system");
  });

  it("toggleDisplayTarget: 成功時に displayTargets が更新される", async () => {
    (commands.setDisplayTargets as Mock).mockResolvedValue({ data: null });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    // 初期状態では displayTargets は空配列
    await act(async () => {
      await result.current.toggleDisplayTarget("cpu");
    });
    expect(result.current.settings.displayTargets).toContain("cpu");

    // 再度呼び出すと対象が外れる（toggle の挙動）
    (commands.setDisplayTargets as Mock).mockResolvedValue({ data: null });
    await act(async () => {
      await result.current.toggleDisplayTarget("cpu");
    });
    expect(result.current.settings.displayTargets).not.toContain("cpu");
  });

  it("toggleDisplayTarget: エラー時に error() が呼ばれ、displayTargets が更新されない", async () => {
    const errorMsg = "Failed to update display targets";
    (commands.setDisplayTargets as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.toggleDisplayTarget("cpu");
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    // エラー時は初期状態（空配列）のまま
    expect(result.current.settings.displayTargets).toEqual([]);
  });

  it("updateLineGraphColorAtom: 成功時に lineGraphColor が更新される", async () => {
    // 例として、"cpu" の色を更新する場合
    (commands.setLineGraphColor as Mock).mockResolvedValue({
      data: "rgb(255,255,255)",
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.updateLineGraphColorAtom("cpu", "#FFFFFF");
    });
    expect(result.current.settings.lineGraphColor.cpu).toEqual(
      "rgb(255,255,255)",
    );
  });

  it("updateLineGraphColorAtom: エラー時に error() が呼ばれ、lineGraphColor が更新されない", async () => {
    const errorMsg = "Failed to update color";
    (commands.setLineGraphColor as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    const initialColor = result.current.settings.lineGraphColor.cpu;
    await act(async () => {
      await result.current.updateLineGraphColorAtom("cpu", "#FFFFFF");
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings.lineGraphColor.cpu).toEqual(initialColor);
  });

  it("updateSettingAtom: 'system' テーマを正常に設定できる", async () => {
    (commands.setTheme as Mock).mockResolvedValue({ data: null });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.updateSettingAtom("theme", "system");
    });

    expect(commands.setTheme).toHaveBeenCalledWith("system");
    expect(result.current.settings.theme).toEqual("system");
  });

  it("デフォルトのテーマが 'system' である", () => {
    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });

    expect(result.current.settings.theme).toEqual("system");
  });
});
