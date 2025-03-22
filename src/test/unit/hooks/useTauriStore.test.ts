import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

interface FakeStore {
  // biome-ignore lint/suspicious/noExplicitAny: <explanation>
  data: Record<string, any>;
  has: (key: string) => Promise<boolean>;
  // biome-ignore lint/suspicious/noExplicitAny: <explanation>
  get: (key: string) => Promise<any>;
  // biome-ignore lint/suspicious/noExplicitAny: <explanation>
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
      // biome-ignore lint/suspicious/noExplicitAny: <explanation>
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

    // 初期状態は useEffect 未実行のため null となるが、非同期処理で更新される
    await waitFor(() => result.current[0] !== null);

    expect(result.current[0]).toBe("storedValue");
    expect(fakeStore.has).toHaveBeenCalledWith("testKey");
    expect(fakeStore.get).toHaveBeenCalledWith("testKey");
  });

  it("初回ロードでキーが存在しない場合、デフォルト値をセットして返す", async () => {
    // Arrange: "nonExisting" は fakeStore.data に存在しない状態
    const { result } = renderHook(() =>
      useTauriStore<string>("nonExisting", "defaultValue"),
    );
    await waitFor(() => result.current[0] !== null);

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
    await waitFor(() => result.current[0] !== null);
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
    await waitFor(() => result.current[0] !== null);
    expect(result.current[0]).toBeUndefined();
  });

  it("読み込み中は isPending が true になっている", async () => {
    const { result } = renderHook(() => useTauriStore("someKey", "someValue"));
    expect(result.current[2]).toBe(true);
    await waitFor(() => result.current[2] === false);
  });
});
