import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useExportToClipboard } from "../../../../../../src/features/hardware/dashboard/hooks/useExportToClipboard";

vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({
  writeText: vi.fn(),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@tauri-apps/api", () => ({
  invoke: vi.fn((cmd) => {
    if (cmd === "getProcessList") {
      return Promise.resolve([
        { pid: 1, name: "Process1" },
        { pid: 2, name: "Process2" },
      ]);
    }
    return Promise.resolve();
  }),
}));

// Mock window.__TAURI_INTERNALS__
Object.defineProperty(window, "__TAURI_INTERNALS__", {
  value: {
    invoke: vi.fn((cmd) => {
      if (cmd === "getProcessList") {
        return Promise.resolve([
          { pid: 1, name: "Process1" },
          { pid: 2, name: "Process2" },
        ]);
      }
      return Promise.resolve();
    }),
  },
});

describe("useExportToClipboard", () => {
  it("should export clipboard content correctly", async () => {
    const { result } = renderHook(() => useExportToClipboard());

    await result.current.exportToClipboard();

    expect(writeText).toHaveBeenCalledWith(expect.any(String));
  });
});
