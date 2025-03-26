// src/test/unit/hooks/useUsageAndHardwareUpdater.test.ts
import { renderHook, waitFor } from "@testing-library/react";
import { Provider, useAtom } from "jotai";
import { type Mock, beforeEach, describe, expect, it, vi } from "vitest";

import { chartConfig } from "@/consts";

import {
  cpuUsageHistoryAtom,
  gpuFanSpeedAtom,
  gpuTempAtom,
  graphicUsageHistoryAtom,
  memoryUsageHistoryAtom,
} from "@/features/hardware/store/chart";

// テスト対象のフック群
import {
  useHardwareUpdater,
  useUsageUpdater,
} from "@/features/hardware/hooks/useHardwareData";

// コマンド群（モック対象）
import { commands } from "@/rspc/bindings";

// ------
// 各コマンドのモック設定
// ------
vi.mock("@/rspc/bindings", () => ({
  commands: {
    getCpuUsage: vi.fn(),
    getMemoryUsage: vi.fn(),
    getGpuUsage: vi.fn(),
    getNvidiaGpuCooler: vi.fn(),
    getGpuTemperature: vi.fn(),
  },
}));

// ------
// テスト本体
// ------

describe("useUsageUpdater", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("CPU使用率の履歴が更新されること", async () => {
    // simulate: commands.getCpuUsage が数値 50 を返す
    (commands.getCpuUsage as Mock).mockResolvedValue(50);

    const { result } = renderHook(
      () => {
        // CPU 使用率履歴の更新フックを呼び出し、同時に atom の値を取得する
        useUsageUpdater("cpu");
        const [history] = useAtom(cpuUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    // useUsageUpdater はマウント時に即座に fetchAndUpdate を呼ぶので、
    // waitFor で atom の更新状態を検証する
    await waitFor(() => {
      const history = result.current;
      expect(history.length).toEqual(chartConfig.historyLengthSec);
      expect(history[history.length - 1]).toEqual(50);
    });
  });

  it("GPU使用率の履歴が更新されること", async () => {
    // simulate: commands.getGpuUsage が { data: 75 } を返す
    (commands.getGpuUsage as Mock).mockResolvedValue({
      status: "ok",
      data: 75,
    });

    const { result } = renderHook(
      () => {
        useUsageUpdater("gpu");
        const [history] = useAtom(graphicUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      const history = result.current;
      expect(history.length).toEqual(chartConfig.historyLengthSec);
      expect(history[history.length - 1]).toEqual(75);
    });
  });

  it("RAM使用率の履歴が更新されること", async () => {
    // simulate: commands.getCpuUsage が数値 50 を返す
    (commands.getMemoryUsage as Mock).mockResolvedValue(50);

    const { result } = renderHook(
      () => {
        // CPU 使用率履歴の更新フックを呼び出し、同時に atom の値を取得する
        useUsageUpdater("memory");
        const [history] = useAtom(memoryUsageHistoryAtom);
        return history;
      },
      { wrapper: Provider },
    );

    // useUsageUpdater はマウント時に即座に fetchAndUpdate を呼ぶので、
    // waitFor で atom の更新状態を検証する
    await waitFor(() => {
      const history = result.current;
      expect(history.length).toEqual(chartConfig.historyLengthSec);
      expect(history[history.length - 1]).toEqual(50);
    });
  });
});

describe("useHardwareUpdater", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("'gpu', 'fan' の場合、gpuFanSpeedAtomが更新される", async () => {
    (commands.getNvidiaGpuCooler as Mock).mockResolvedValue({
      status: "ok",
      data: [{ name: "test1", value: 100 }],
    });

    const { result } = renderHook(
      () => {
        useHardwareUpdater("gpu", "fan");
        const [data] = useAtom(gpuFanSpeedAtom);
        return data;
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      expect(result.current).toEqual([{ name: "test1", value: 100 }]);
    });
  });

  it("'gpu', 'temp' の場合、gpuTempAtomが更新される", async () => {
    (commands.getGpuTemperature as Mock).mockResolvedValue({
      status: "ok",
      data: [{ name: "test2", value: 70 }],
    });

    const { result } = renderHook(
      () => {
        useHardwareUpdater("gpu", "temp");
        const [data] = useAtom(gpuTempAtom);
        return data;
      },
      { wrapper: Provider },
    );

    await waitFor(() => {
      expect(result.current).toEqual([{ name: "test2", value: 70 }]);
    });
  });
});
