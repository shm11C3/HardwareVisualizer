import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  setCloseToTrayPreference: vi.fn(),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    setCloseToTrayPreference: mocks.setCloseToTrayPreference,
  },
}));

describe("setCloseToTrayPreference", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.setCloseToTrayPreference.mockResolvedValue({
      data: null,
      status: "ok",
    });
  });

  it("persists preference through the settings IPC", async () => {
    const { setCloseToTrayPreference } = await import(
      "./useCloseToTrayPreference"
    );

    const saved = await setCloseToTrayPreference(true);

    expect(saved).toBe(true);
    expect(mocks.setCloseToTrayPreference).toHaveBeenCalledWith(true);
  });

  it("returns false when saving fails", async () => {
    const { setCloseToTrayPreference } = await import(
      "./useCloseToTrayPreference"
    );
    mocks.setCloseToTrayPreference.mockResolvedValueOnce({
      error: "save failed",
      status: "error",
    });

    await expect(setCloseToTrayPreference(true)).resolves.toBe(false);
  });
});
