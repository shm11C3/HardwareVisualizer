import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

interface FakeStore {
  // biome-ignore lint/suspicious/noExplicitAny: using any for test flexibility
  data: Record<string, any>;
  has: (key: string) => Promise<boolean>;
  // biome-ignore lint/suspicious/noExplicitAny: using any for test flexibility
  get: (key: string) => Promise<any>;
  // biome-ignore lint/suspicious/noExplicitAny: using any for test flexibility
  set: (key: string, value: any) => Promise<void>;
  save: () => Promise<void>;
}

// この変数にテスト毎に新しいストアオブジェクトを代入する
let fakeStore: FakeStore;

// useTauriStore フックの再読み込み用変数
let useTauriStore: <T>(
  key: string,
  defaultValue: T,
) => [T | null, (newValue: T) => Promise<void>, boolean];

describe("useTauriStore", () => {
  beforeEach(async () => {
    // モジュールキャッシュをクリアして再読み込みできるようにする
    vi.resetModules();

    // テスト毎に新しい fakeStore を作成
    fakeStore = {
      data: {},
      has: vi.fn((key: string) => Promise.resolve(key in fakeStore.data)),
      get: vi.fn((key: string) => Promise.resolve(fakeStore.data[key])),
      // biome-ignore lint/suspicious/noExplicitAny: using any for test flexibility
      set: vi.fn((key: string, value: any) => {
        fakeStore.data[key] = value;
        return Promise.resolve();
      }),
      save: vi.fn(() => Promise.resolve()),
    };

    // @tauri-apps/plugin-store モジュールをモック化する
    // storePromise（load("store.json", { autoSave: true })）の返り値として fakeStore を返す
    vi.doMock("@tauri-apps/plugin-store", () => ({
      load: vi.fn(() => Promise.resolve(fakeStore)),
    }));

    // useTauriStore を含むモジュールを再読み込みする
    const module = await import("@/hooks/useTauriStore");
    useTauriStore = <T>(key: string, defaultValue: T) => {
      return module.useTauriStore<T>(key, defaultValue);
    };
  });

  afterEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
  });

  it("初回ロードでキーが存在する場合、ストアから取得した値が返る", async () => {
    // Arrange: fakeStore に既に値が保存されている状態
    fakeStore.data.testKey = "storedValue";
    const { result } = renderHook(() =>
      useTauriStore<string>("testKey", "defaultValue"),
    );

    // React 19.1.0 対応: act でラップして確実に状態更新を完了
    await act(async () => {
      await waitFor(() => !result.current[2] && result.current[0] === "storedValue");
    });

      const [value, , isPending] = result.current;
      await waitFor(() => !isPending && value === "storedValue");
    });

    const [value] = result.current;
    expect(value).toBe("storedValue");
    expect(fakeStore.has).toHaveBeenCalledWith("testKey");
    expect(fakeStore.get).toHaveBeenCalledWith("testKey");
  });

  it("初回ロードでキーが存在しない場合、デフォルト値をセットして返す", async () => {
    // Arrange: "nonExisting" は fakeStore.data に存在しない状態
    const { result } = renderHook(() =>
      useTauriStore<string>("nonExisting", "defaultValue"),
    );
    
    // React 19.1.0 対応: act でラップして確実に状態更新を完了
    await act(async () => {
      await waitFor(() => !result.current[2] && result.current[0] === "defaultValue");
    });

    // キーが存在しなかったため、defaultValue がセットされる
    expect(result.current[0]).toBe("defaultValue");
    // 存在しなかったので set と save が呼ばれている
    expect(fakeStore.set).toHaveBeenCalledWith("nonExisting", "defaultValue");
    expect(fakeStore.save).toHaveBeenCalled();
  });

  it("setValue を呼び出すと、新しい値が更新される", async () => {
    const { result } = renderHook(() =>
      useTauriStore<string>("testKey", "defaultValue"),
    );
    
    // React 19.1.0 対応: act でラップして確実に初期状態更新を完了
    await act(async () => {
      await waitFor(() => !result.current[2] && result.current[0] === "defaultValue");
    });
    expect(result.current[0]).toBe("defaultValue");

    // Act: setValue を呼び出して値を更新する
    await act(async () => {
      await result.current[1]("newValue");
    });

    // Assert: state が新しい値に更新される
    expect(result.current[0]).toBe("newValue");
    expect(fakeStore.set).toHaveBeenCalledWith("testKey", "newValue");
    expect(fakeStore.save).toHaveBeenCalled();
  });

  it("undefined の defaultValue を扱える", async () => {
    const { result } = renderHook(() =>
      useTauriStore<undefined>("testKey", undefined),
    );
    
    // React 19.1.0 対応: act でラップして確実に状態更新を完了
    await act(async () => {
      await waitFor(() => !result.current[2]);
    });
    
    expect(result.current[0]).toBeUndefined();
  });

  it("読み込み中は isPending が true になっている", async () => {
    const { result } = renderHook(() => useTauriStore("someKey", "someValue"));
    expect(result.current[2]).toBe(true);
    await waitFor(() => result.current[2] === false);
  });
});
