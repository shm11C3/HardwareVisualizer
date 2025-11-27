import {
  ask as showAsk,
  confirm as showConfirm,
  message as showMessage,
} from "@tauri-apps/plugin-dialog";
import { renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";
import { useTauriDialog } from "@/hooks/useTauriDialog";

// Mock the useTranslation hook from react-i18next
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    // For testing, simply return the translation key as-is
    t: (key: string) => key,
  }),
}));

// Mock each function from @tauri-apps/plugin-dialog
vi.mock("@tauri-apps/plugin-dialog", () => ({
  ask: vi.fn(),
  confirm: vi.fn(),
  message: vi.fn(),
}));

describe("useTauriDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("ask: When title is specified, calls showAsk with translated title", async () => {
    const { result } = renderHook(() => useTauriDialog());
    // Set mock return value (e.g., true)
    (showAsk as Mock).mockResolvedValue(true);

    const dialog = result.current;
    const res = await dialog.ask({
      title: "info",
      message: "Are you sure?",
      kind: "warning",
    });

    expect(showAsk).toHaveBeenCalledWith("Are you sure?", {
      title: "error.title.info",
      kind: "warning",
    });
    expect(res).toBe(true);
  });

  it("ask: タイトルが指定されていない場合、title は undefined となる", async () => {
    const { result } = renderHook(() => useTauriDialog());
    (showAsk as Mock).mockResolvedValue(false);

    const dialog = result.current;
    const res = await dialog.ask({
      message: "Proceed?",
      kind: "error",
    });

    expect(showAsk).toHaveBeenCalledWith("Proceed?", {
      title: undefined,
      kind: "error",
    });
    expect(res).toBe(false);
  });

  it("confirm: タイトルが指定されている場合、翻訳されたタイトルとともに showConfirm を呼び出す", async () => {
    const { result } = renderHook(() => useTauriDialog());
    (showConfirm as Mock).mockResolvedValue(true);

    const dialog = result.current;
    const res = await dialog.confirm({
      title: "confirm",
      message: "Do you want to delete?",
      kind: "warning",
    });

    expect(showConfirm).toHaveBeenCalledWith("Do you want to delete?", {
      title: "error.title.confirm",
      kind: "warning",
    });
    expect(res).toBe(true);
  });

  it("message: タイトルが指定されている場合、翻訳されたタイトルとともに showMessage を呼び出す", async () => {
    const { result } = renderHook(() => useTauriDialog());
    (showMessage as Mock).mockResolvedValue(undefined);

    const dialog = result.current;
    await dialog.message({
      title: "success",
      message: "Operation completed",
      kind: "info",
    });

    expect(showMessage).toHaveBeenCalledWith("Operation completed", {
      title: "error.title.success",
      kind: "info",
    });
  });

  it("error: エラー時は、内部的に message を呼び出し、タイトルに 'error' を指定する", async () => {
    const { result } = renderHook(() => useTauriDialog());
    (showMessage as Mock).mockResolvedValue(undefined);

    const dialog = result.current;
    await dialog.error("An unexpected error occurred");

    expect(showMessage).toHaveBeenCalledWith("An unexpected error occurred", {
      title: "error.title.error",
      kind: "error",
    });
  });
});
