import { useUploadImage } from "@/components/forms/UploadImage/useUploadImageForm";
import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

if (!URL.createObjectURL) {
  URL.createObjectURL = () => "blob:testurl";
}

// --- 依存モジュールのモック設定 ---
// react-i18next
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

// useTauriDialog のモック化
const errorMock = vi.fn();
vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: errorMock,
  }),
}));

// useBackgroundImage のモック化
const saveBackgroundImageMock = vi.fn();
vi.mock("@/hooks/useBgImage", () => ({
  useBackgroundImage: () => ({
    saveBackgroundImage: saveBackgroundImageMock,
  }),
}));

// --- URL.createObjectURL のモック化 ---
const createObjectURLSpy = vi
  .spyOn(URL, "createObjectURL")
  .mockReturnValue("blob:testurl");

describe("useUploadImage", () => {
  beforeEach(() => {
    errorMock.mockReset();
    saveBackgroundImageMock.mockReset();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("初期状態では fileName が空文字、displayUrl が null、isSubmitting が false となる", () => {
    const { result } = renderHook(() => useUploadImage());
    expect(result.current.fileName).toBe("");
    expect(result.current.displayUrl).toBeNull();
    expect(result.current.isSubmitting).toBe(false);
  });

  it("picture にファイルがセットされると fileName と displayUrl が更新される", async () => {
    const fakeFile = new File(["dummy content"], "test.png", {
      type: "image/png",
    });
    const { result } = renderHook(() => useUploadImage());

    // form は react-hook-form のオブジェクトなので setValue で picture を更新できる
    act(() => {
      result.current.form.setValue("picture", fakeFile);
    });

    // useEffect の非同期更新を待つ
    await waitFor(() => result.current.fileName !== "");

    expect(result.current.fileName).toBe("test.png");
    // URL.createObjectURL に渡されたファイルが正しいかも確認（モックの戻り値 "blob:testurl"）
    expect(result.current.displayUrl).toBe("blob:testurl");
    expect(createObjectURLSpy).toHaveBeenCalledWith(fakeFile);
  });

  it("onSubmit 呼び出しで正常に画像が保存され、フォームがリセットされる", async () => {
    const fakeFile = new File(["dummy content"], "test.jpg", {
      type: "image/jpeg",
    });
    const { result } = renderHook(() => useUploadImage());

    // まず、picture に値をセット
    act(() => {
      result.current.form.setValue("picture", fakeFile);
    });

    // 送信前は isSubmitting が false
    expect(result.current.isSubmitting).toBe(false);

    // onSubmit を呼び出す（正常ケース）
    await act(async () => {
      await result.current.onSubmit({ picture: fakeFile });
    });

    // saveBackgroundImageMock が正しく呼ばれたことを検証
    expect(saveBackgroundImageMock).toHaveBeenCalledWith(fakeFile);
    // フォームがリセットされ、picture の値が undefined（defaultValues）になっているはず
    expect(result.current.form.getValues("picture")).toBeUndefined();
    // isSubmitting が処理完了後に false に戻る
    expect(result.current.isSubmitting).toBe(false);
  });

  it("onSubmit で saveBackgroundImage がエラーの場合、error ハンドラが呼ばれる", async () => {
    const fakeFile = new File(["dummy content"], "error.png", {
      type: "image/png",
    });
    // saveBackgroundImageMock をエラー返却に上書き
    saveBackgroundImageMock.mockRejectedValue(new Error("Test error"));
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    const { result } = renderHook(() => useUploadImage());

    // picture に値をセット
    act(() => {
      result.current.form.setValue("picture", fakeFile);
    });

    await act(async () => {
      await result.current.onSubmit({ picture: fakeFile });
    });

    // エラー時に error ハンドラが呼ばれていることを検証
    expect(errorMock).toHaveBeenCalledWith(new Error("Test error"));
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "Error saveBackgroundImage:",
      expect.any(Error),
    );
    // isSubmitting は false に戻っている
    expect(result.current.isSubmitting).toBe(false);
    consoleErrorSpy.mockRestore();
  });
});
