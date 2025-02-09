import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { type Mock, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * モックの設定
 */
const errorMock = vi.fn();
vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: errorMock,
  }),
}));

// commands 内の getHardwareInfo, getNetworkInfo をモック化
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getHardwareInfo: vi.fn(),
    getNetworkInfo: vi.fn(),
  },
}));

/**
 * テスト対象のフックをインポート
 */
import { useHardwareInfoAtom } from "@/atom/useHardwareInfoAtom";
import { commands } from "@/rspc/bindings";

/**
 * テスト実行
 */
describe("useHardwareInfoAtom", () => {
  beforeEach(() => {
    // 各テスト実行前にモック状態をリセット
    vi.clearAllMocks();
  });

  it("init: 成功時に hardwareInfo が更新される", async () => {
    // コマンドから返すデータをモック
    const hardwareData = {
      cpu: "Intel",
      memory: "16GB",
      gpus: "NVIDIA",
      storage: ["SSD"],
    };
    (commands.getHardwareInfo as Mock).mockResolvedValue({
      data: hardwareData,
    });

    // Provider でラップしてフックをレンダリング
    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    // async の中で act() を使用して init() を実行
    await act(async () => {
      await result.current.init();
    });

    // hardwareInfo が更新されていることを確認
    expect(result.current.hardwareInfo).toEqual(hardwareData);
  });

  it("init: エラー時に error() が呼ばれ、hardwareInfo は初期値のまま", async () => {
    const errorMsg = "Failed to fetch hardware info";
    (commands.getHardwareInfo as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.init();
    });

    // error() が呼ばれていること
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    // 初期状態（cpu, memory, gpus が null、storage が空配列）のまま
    expect(result.current.hardwareInfo).toEqual({
      cpu: null,
      memory: null,
      gpus: null,
      storage: [],
    });
    expect(consoleErrorSpy).toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });

  it("initNetwork: 成功時に networkInfo が更新される", async () => {
    const networkData = [{ name: "eth0", ip: "192.168.1.2" }];
    (commands.getNetworkInfo as Mock).mockResolvedValue({ data: networkData });

    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.initNetwork();
    });

    expect(result.current.networkInfo).toEqual(networkData);
  });

  it("initNetwork: エラー時に error() が呼ばれ、networkInfo は初期値のまま", async () => {
    const errorMsg = "Failed to fetch network info";
    (commands.getNetworkInfo as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    const { result } = renderHook(() => useHardwareInfoAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.initNetwork();
    });

    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.networkInfo).toEqual([]);
    expect(consoleErrorSpy).toHaveBeenCalled();
    consoleErrorSpy.mockRestore();
  });
});
