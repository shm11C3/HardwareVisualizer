import { message } from "@tauri-apps/plugin-dialog";
import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useErrorModalListener } from "@/hooks/useTauriEventListener";

// --- モックの設定 ---
// テスト内でイベントリスナーのコールバックを保持するための変数
let registeredCallback:
  | ((event: { payload: { title: string; message: string } }) => void)
  | undefined;

// イベントリスナー解除用のモック関数
const offMock = vi.fn();

// Tauri の listen をモック化
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(
    (
      _eventName: string,
      handler: (event: { payload: { title: string; message: string } }) => void,
    ) => {
      // 登録されたコールバック関数を保持
      registeredCallback = handler;
      // オフ関数を返す Promise を返す
      return Promise.resolve(offMock);
    },
  ),
}));

// Tauri の message をモック化
vi.mock("@tauri-apps/plugin-dialog", () => ({
  message: vi.fn(),
}));

describe("useErrorModalListener", () => {
  beforeEach(() => {
    // 各テストの前にモックの状態をリセット
    vi.clearAllMocks();
    registeredCallback = undefined;
  });

  it("エラーイベント受信時に正しくダイアログが表示される", async () => {
    // フックを実行
    renderHook(() => useErrorModalListener());

    // 登録されたコールバックが存在するか確認
    if (!registeredCallback) {
      throw new Error("イベントリスナーが登録されていません");
    }

    // エラーイベントをシミュレーション
    const errorPayload = { title: "Error Title", message: "An error occurred" };
    registeredCallback({ payload: errorPayload });

    // イベントハンドラ内の非同期処理完了待ち
    await Promise.resolve();

    // エラー内容が正しくダイアログ表示用の message 関数に渡されていることを検証
    expect(message).toHaveBeenCalledWith("An error occurred", {
      title: "Error Title",
      kind: "error",
    });
  });

  it("コンポーネントのアンマウント時にイベントリスナーが解除される", async () => {
    const { unmount } = renderHook(() => useErrorModalListener());

    // フックのクリーンアップ（アンマウント）を実行
    unmount();

    // 非同期処理の完了を待つ（Promise の解決）
    await Promise.resolve();

    // イベントリスナー解除用のオフ関数が呼ばれていることを検証
    expect(offMock).toHaveBeenCalled();
  });
});
