// src/test/unit/hooks/useBgImage.test.ts
import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { type Mock, beforeEach, describe, expect, it, vi } from "vitest";

// ----------------------
// モックの設定
// ----------------------

// Tauri のダイアログ用エラー関数をモック化
const errorMock = vi.fn();
vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: errorMock }),
}));

// useSettingsAtom のモック用に、グローバル変数で設定状態と更新関数を管理
let settingsMock: { selectedBackgroundImg: string | null } = {
  selectedBackgroundImg: null,
};
let updateSettingAtomMock = vi.fn();

// 各テスト毎に状態をリセット
beforeEach(() => {
  vi.clearAllMocks();
  settingsMock = { selectedBackgroundImg: null };
  updateSettingAtomMock = vi.fn();
});

// useSettingsAtom をモック化（テスト毎に変更可能なグローバル変数を参照）
vi.mock("@/atom/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: settingsMock,
    updateSettingAtom: updateSettingAtomMock,
  }),
}));

// ファイル変換用関数のモック化（トップレベル変数に依存せず、直接 vi.fn() を返す）
vi.mock("@/lib/file", () => ({
  convertFileToBase64: vi.fn(),
}));

// コマンド群をモック化
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getBackgroundImage: vi.fn(),
    saveBackgroundImage: vi.fn(),
    deleteBackgroundImage: vi.fn(),
    getBackgroundImages: vi.fn(),
  },
}));

// ----------------------
// モック関数を利用するために、対象モジュールからインポートし、vi.mocked を利用
// ----------------------
import { convertFileToBase64 } from "@/lib/file";
const convertFileToBase64Mock = vi.mocked(convertFileToBase64);

import { useBackgroundImage, useBackgroundImageList } from "@/hooks/useBgImage";
import { commands } from "@/rspc/bindings";

// ----------------------
// テスト本体
// ----------------------
describe("useBackgroundImage", () => {
  beforeEach(() => {
    errorMock.mockClear();
    updateSettingAtomMock.mockClear();
    convertFileToBase64Mock.mockClear();
    (commands.getBackgroundImages as Mock).mockResolvedValue({ data: [] });
  });

  describe("initBackgroundImage", () => {
    it("settings.selectedBackgroundImgが存在し、getBackgroundImageがokを返した場合、backgroundImageを設定する", async () => {
      // settings に背景画像IDが設定されている場合
      settingsMock.selectedBackgroundImg = "file123";
      // getBackgroundImage が正常結果を返す
      (commands.getBackgroundImage as Mock).mockResolvedValue({
        status: "ok",
        data: "base64data",
      });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.initBackgroundImage();
      });

      expect(result.current.backgroundImage).toEqual(
        "data:image/png;base64,base64data",
      );
    });

    it("getBackgroundImageがエラーを返したときにエラーを呼び出す", async () => {
      settingsMock.selectedBackgroundImg = "file123";
      const errorMsg = "Error loading image";
      (commands.getBackgroundImage as Mock).mockResolvedValue({
        status: "error",
        error: errorMsg,
      });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.initBackgroundImage();
      });

      expect(errorMock).toHaveBeenCalledWith(errorMsg);
      // 背景画像が設定されていない（null）状態となる
      expect(result.current.backgroundImage).toBeNull();
    });

    it("selectedBackgroundImgが設定されていない場合、backgroundImageをnullに設定する", async () => {
      settingsMock.selectedBackgroundImg = null;

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.initBackgroundImage();
      });

      expect(result.current.backgroundImage).toBeNull();
    });
  });

  describe("saveBackgroundImage", () => {
    it("保存成功後に設定が更新されること", async () => {
      // ダミーのファイルオブジェクトを生成（File コンストラクタが利用可能なテスト環境で）
      const file = new File(["dummy content"], "dummy.png", {
        type: "image/png",
      });

      // convertFileToBase64 が base64 文字列を返す
      convertFileToBase64Mock.mockResolvedValue("base64image");

      // saveBackgroundImage が成功結果を返す
      (commands.saveBackgroundImage as Mock).mockResolvedValue({
        status: "ok",
        data: "newFileId",
      });

      // useBackgroundImage 内で useBackgroundImageList から initBackgroundImages を利用しているので、
      // その関数が呼ばれたかどうかをスパイする
      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.saveBackgroundImage(file);
      });

      // updateSettingAtom が呼ばれて新しい背景画像IDが設定されることを検証
      expect(updateSettingAtomMock).toHaveBeenCalledWith(
        "selectedBackgroundImg",
        "newFileId",
      );
    });

    it("saveBackgroundImageがエラーを返した場合、エラーを呼び出し、設定を更新は更新されない", async () => {
      const file = new File(["dummy content"], "dummy.png", {
        type: "image/png",
      });

      convertFileToBase64Mock.mockResolvedValue("base64image");

      const errorMsg = "Save failed";
      (commands.saveBackgroundImage as Mock).mockResolvedValue({
        status: "error",
        error: errorMsg,
      });

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.saveBackgroundImage(file);
      });

      expect(errorMock).toHaveBeenCalledWith(errorMsg);
      expect(updateSettingAtomMock).not.toHaveBeenCalled();
    });
  });

  describe("deleteBackgroundImage", () => {
    it("選択された背景画像が削除され、設定がリセットされることを確認", async () => {
      // settings で削除対象が選択されている状態にする
      settingsMock.selectedBackgroundImg = "fileToDelete";

      // 初期の背景画像リストを設定
      const initialList = [
        { fileId: "fileToDelete", imageData: "data:image/png;base64,oldData" },
        { fileId: "otherFile", imageData: "data:image/png;base64,otherData" },
      ];

      // useBackgroundImageList のフックを個別にレンダリングしてリストを操作
      const { result: listResult } = renderHook(
        () => useBackgroundImageList(),
        { wrapper: Provider },
      );
      act(() => {
        listResult.current.setBackgroundImageList(initialList);
      });

      // deleteBackgroundImage が成功するように設定
      (commands.deleteBackgroundImage as Mock).mockResolvedValue({});

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.deleteBackgroundImage("fileToDelete");
      });

      // 選択中だったため、updateSettingAtom が null をセットする
      expect(updateSettingAtomMock).toHaveBeenCalledWith(
        "selectedBackgroundImg",
        null,
      );
    });

    it("エラーをスローしたときにエラーダイアログを呼び出すようにする", async () => {
      // 初期の背景画像リスト
      const initialList = [
        { fileId: "fileToDelete", imageData: "data:image/png;base64,oldData" },
      ];

      const { result: listResult } = renderHook(
        () => useBackgroundImageList(),
        { wrapper: Provider },
      );
      act(() => {
        listResult.current.setBackgroundImageList(initialList);
      });

      const errorMsg = "Deletion failed";
      // deleteBackgroundImage が例外をスローするように設定
      (commands.deleteBackgroundImage as Mock).mockRejectedValue(errorMsg);

      const { result } = renderHook(() => useBackgroundImage(), {
        wrapper: Provider,
      });

      await act(async () => {
        await result.current.deleteBackgroundImage("fileToDelete");
      });

      expect(errorMock).toHaveBeenCalledWith(errorMsg);
    });
  });
});

describe("useBackgroundImageList", () => {
  beforeEach(() => {
    errorMock.mockClear();
  });

  it("initBackgroundImages: getBackgroundImagesが成功を返したら、backgroundImageListを更新する。", async () => {
    // getBackgroundImages が複数の画像データを返す
    const images = [
      { fileId: "img1", imageData: "imgData1" },
      { fileId: "img2", imageData: "imgData2" },
    ];
    (commands.getBackgroundImages as Mock).mockResolvedValue({
      status: "ok",
      data: images,
    });

    const { result } = renderHook(() => useBackgroundImageList(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.initBackgroundImages();
    });

    // 返された各画像の imageData にプレフィックスが付与されることを確認
    expect(result.current.backgroundImageList).toEqual([
      { fileId: "img1", imageData: "data:image/png;base64,imgData1" },
      { fileId: "img2", imageData: "data:image/png;base64,imgData2" },
    ]);
  });

  it("initBackgroundImages: getBackgroundImagesがエラーを返した場合、エラーをコールする", async () => {
    const errorMsg = "Failed to load images";
    (commands.getBackgroundImages as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useBackgroundImageList(), {
      wrapper: Provider,
    });
    // 初期状態を空リストにしておく
    act(() => {
      result.current.setBackgroundImageList([]);
    });

    await act(async () => {
      await result.current.initBackgroundImages();
    });

    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.backgroundImageList).toEqual([]);
  });
});
