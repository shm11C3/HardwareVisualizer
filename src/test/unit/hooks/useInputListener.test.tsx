import { useKeydown } from "@/hooks/useInputListener";
import { commands } from "@/rspc/bindings";
import { cleanup, render } from "@testing-library/react";
import {
  type Mock,
  afterEach,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";

// --- モックの定義 ---
// useTauriDialog の error 関数をモック
const errorMock = vi.fn();

// useTauriStore の戻り値として [状態, setState] を返すモック
let storeValue = false;
const setDecoratedMock = vi.fn((newVal: boolean) => {
  storeValue = newVal;
  return Promise.resolve();
});

// useTauriDialog をモック化
vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: errorMock,
  }),
}));
// useTauriStore をモック化
vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: (_key: string, defaultValue: boolean) => {
    storeValue = defaultValue;
    return [storeValue, setDecoratedMock];
  },
}));

// commands.setDecoration をモック化
vi.mock("@/rspc/bindings", () => ({
  commands: {
    setDecoration: vi.fn(() => Promise.resolve()),
  },
}));

// テスト用のコンポーネント
function TestComponent() {
  // Pass the required arguments to useKeydown
  const isDecorated = storeValue;
  const keydownHandler = useKeydown({
    isDecorated,
    setDecorated: setDecoratedMock,
  });
  return (
    <>
      {keydownHandler}
      <div>Test Component</div>
    </>
  );
}

describe("useKeydown", () => {
  beforeEach(() => {
    // 初期状態のリセット
    storeValue = false;
    errorMock.mockReset();
    setDecoratedMock.mockReset();
    (commands.setDecoration as Mock).mockClear();
  });

  afterEach(() => {
    cleanup();
  });

  it("F11 キー押下時に commands.setDecoration と setDecorated が呼ばれる", async () => {
    render(<TestComponent />);
    // F11 キーのイベントをディスパッチ
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F11" }));
    // 非同期処理完了待ち（マイクロタスクの完了を待つ）
    await new Promise((resolve) => setTimeout(resolve, 0));

    // 初期状態 false なので !false → true が渡されることを検証
    expect(commands.setDecoration).toHaveBeenCalledWith(true);
    expect(setDecoratedMock).toHaveBeenCalledWith(true);
  });

  it("F11 以外のキー押下時は何も実行されない", async () => {
    render(<TestComponent />);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(commands.setDecoration).not.toHaveBeenCalled();
    expect(setDecoratedMock).not.toHaveBeenCalled();
  });

  it("commands.setDecoration でエラーが発生した場合、error ハンドラと console.error が呼ばれる", async () => {
    // commands.setDecoration がエラーを返すように実装
    const errorMessage = "Test error";
    (commands.setDecoration as Mock).mockImplementationOnce(() =>
      Promise.reject(new Error(errorMessage)),
    );
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    render(<TestComponent />);
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F11" }));
    await new Promise((resolve) => setTimeout(resolve, 0));

    // error モックと console.error が呼ばれていることを確認
    expect(errorMock).toHaveBeenCalled();
    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "Failed to toggle window decoration:",
      expect.any(Error),
    );

    consoleErrorSpy.mockRestore();
  });

  it("コンポーネントのアンマウント時にイベントリスナーが削除される", async () => {
    const { unmount } = render(<TestComponent />);
    unmount();

    // アンマウント後に F11 イベントを発生させても何も起こらないことを検証
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "F11" }));
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(commands.setDecoration).not.toHaveBeenCalled();
    expect(setDecoratedMock).not.toHaveBeenCalled();
  });
});
