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

  it("When a valid URL is passed, open is called", async () => {
    const validURL = "https://example.com";

    // Execute openURL
    await openURL(validURL);

    // Verify that open was called with validURL
    expect(open).toHaveBeenCalledWith(validURL);
  });

  it("When an invalid URL is passed, an error is thrown", async () => {
    const invalidURL = "invalid-url";

    // Since creating new URL(url) throws an error for invalidURL, openURL should throw an error
    await expect(openURL(invalidURL)).rejects.toThrow("Invalid URL");

    // open should not be called
    expect(open).not.toHaveBeenCalled();
  });
});
