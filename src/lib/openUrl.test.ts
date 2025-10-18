import { open } from "@tauri-apps/plugin-shell";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { openURL } from "@/lib/openUrl";

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: vi.fn(),
}));

describe("openURL", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("有効な URL が渡された場合、open が呼ばれる", async () => {
    const validURL = "https://example.com";

    // openURL を実行
    await openURL(validURL);

    // open が validURL で呼ばれたことを検証
    expect(open).toHaveBeenCalledWith(validURL);
  });

  it("無効な URL が渡された場合、エラーがスローされる", async () => {
    const invalidURL = "invalid-url";

    // invalidURL では新しい URL(url) 生成時にエラーが発生するため、openURL はエラーを throw する
    await expect(openURL(invalidURL)).rejects.toThrow("Invalid URL");

    // open は呼ばれないはず
    expect(open).not.toHaveBeenCalled();
  });
});
