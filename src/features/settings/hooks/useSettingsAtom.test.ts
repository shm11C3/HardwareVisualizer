import { act, renderHook } from "@testing-library/react";
import { Provider } from "jotai";
// src/features/settings/hooks/useSettingsAtom.test.ts
import { beforeEach, describe, expect, it, type Mock, vi } from "vitest";

/**
 * Mock setup
 */
const errorMock = vi.fn();

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: errorMock,
  }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getSettings: vi.fn(),
    setTheme: vi.fn(),
    setDisplayTargets: vi.fn(),
    setGraphSize: vi.fn(),
    setLineGraphType: vi.fn(),
    setLanguage: vi.fn(),
    setLineGraphBorder: vi.fn(),
    setLineGraphFill: vi.fn(),
    setLineGraphMix: vi.fn(),
    setLineGraphShowLegend: vi.fn(),
    setLineGraphShowScale: vi.fn(),
    setLineGraphShowTooltip: vi.fn(),
    setBackgroundImgOpacity: vi.fn(),
    setSelectedBackgroundImg: vi.fn(),
    setTemperatureUnit: vi.fn(),
    setLineGraphColor: vi.fn(),
    setBurnInShift: vi.fn(),
    setBurnInShiftPreset: vi.fn(),
    setBurnInShiftMode: vi.fn(),
    setBurnInShiftIdleOnly: vi.fn(),
    setBurnInShiftOptions: vi.fn(),
    setHardwareArchiveEnabled: vi.fn(),
    setHardwareArchiveInterval: vi.fn(),
    setHardwareArchiveScheduledDataDeletion: vi.fn(),
  },
}));

/**
 * Import hook to test
 */
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { commands } from "@/rspc/bindings";

/**
 * Test execution
 */
describe("useSettingsAtom", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("loadSettings: settings is updated on success", async () => {
    // Test settings data
    const settingsData = {
      version: "1.0.0",
      language: "ja",
      theme: "dark",
      displayTargets: ["cpu"],
      graphSize: "lg",
      lineGraphType: "custom",
      lineGraphBorder: false,
      lineGraphFill: false,
      lineGraphColor: {
        cpu: "rgb(255,0,0)",
        memory: "rgb(0,255,0)",
        gpu: "rgb(0,0,255)",
      },
      lineGraphMix: false,
      lineGraphShowLegend: false,
      lineGraphShowScale: true,
      lineGraphShowTooltip: false,
      backgroundImgOpacity: 70,
      selectedBackgroundImg: "image.png",
      temperatureUnit: "F",
    };

    // Mock commands.getSettings to return success result
    (commands.getSettings as Mock).mockResolvedValue({ data: settingsData });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.loadSettings();
    });
    expect(result.current.settings).toEqual(settingsData);
  });

  it("loadSettings: error() is called on error and settings remains at initial value", async () => {
    const errorMsg = "Failed to fetch settings";
    (commands.getSettings as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    // Save initial state before loadSettings
    const initialSettings = result.current.settings;
    await act(async () => {
      await result.current.loadSettings();
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings).toEqual(initialSettings);
  });

  it("updateSettingAtom: settings is updated on success", async () => {
    // Test "theme" update as example
    (commands.setTheme as Mock).mockResolvedValue({ data: null });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    // Initially theme is "light" (settingsAtom default value)
    await act(async () => {
      await result.current.updateSettingAtom("theme", "dark");
    });
    expect(result.current.settings.theme).toEqual("dark");
  });

  it("updateSettingAtom: error() is called on error and value is reverted", async () => {
    const errorMsg = "Failed to update theme";
    (commands.setTheme as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    // Initial theme is "system"
    await act(async () => {
      await result.current.updateSettingAtom("theme", "dark");
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    // On failure, reverts to original value ("system")
    expect(result.current.settings.theme).toEqual("system");
  });

  it("toggleDisplayTarget: displayTargets is updated on success", async () => {
    (commands.setDisplayTargets as Mock).mockResolvedValue({ data: null });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    // Initially displayTargets is empty array
    await act(async () => {
      await result.current.toggleDisplayTarget("cpu");
    });
    expect(result.current.settings.displayTargets).toContain("cpu");

    // Calling again removes target (toggle behavior)
    (commands.setDisplayTargets as Mock).mockResolvedValue({ data: null });
    await act(async () => {
      await result.current.toggleDisplayTarget("cpu");
    });
    expect(result.current.settings.displayTargets).not.toContain("cpu");
  });

  it("toggleDisplayTarget: error() is called on error and displayTargets is not updated", async () => {
    const errorMsg = "Failed to update display targets";
    (commands.setDisplayTargets as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.toggleDisplayTarget("cpu");
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    // On error, remains at initial state (empty array)
    expect(result.current.settings.displayTargets).toEqual([]);
  });

  it("updateLineGraphColorAtom: lineGraphColor is updated on success", async () => {
    // Test updating "cpu" color as example
    (commands.setLineGraphColor as Mock).mockResolvedValue({
      data: "rgb(255,255,255)",
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.updateLineGraphColorAtom("cpu", "#FFFFFF");
    });
    expect(result.current.settings.lineGraphColor.cpu).toEqual(
      "rgb(255,255,255)",
    );
  });

  it("updateLineGraphColorAtom: error() is called on error and lineGraphColor is not updated", async () => {
    const errorMsg = "Failed to update color";
    (commands.setLineGraphColor as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    const initialColor = result.current.settings.lineGraphColor.cpu;
    await act(async () => {
      await result.current.updateLineGraphColorAtom("cpu", "#FFFFFF");
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings.lineGraphColor.cpu).toEqual(initialColor);
  });

  it("updateSettingAtom: 'system' theme can be set successfully", async () => {
    (commands.setTheme as Mock).mockResolvedValue({ data: null });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.updateSettingAtom("theme", "system");
    });

    expect(commands.setTheme).toHaveBeenCalledWith("system");
    expect(result.current.settings.theme).toEqual("system");
  });

  it("Default theme is 'system'", () => {
    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });

    expect(result.current.settings.theme).toEqual("system");
  });

  it("toggleHardwareArchiveAtom: hardwareArchive.enabled is updated on success", async () => {
    (commands.setHardwareArchiveEnabled as Mock).mockResolvedValue({
      data: null,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.toggleHardwareArchiveAtom(false);
    });
    expect(result.current.settings.hardwareArchive.enabled).toBe(false);
  });

  it("toggleHardwareArchiveAtom: error() is called on error and hardwareArchive is not updated", async () => {
    const errorMsg = "Failed to toggle archive";
    (commands.setHardwareArchiveEnabled as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    const initialEnabled = result.current.settings.hardwareArchive.enabled;
    await act(async () => {
      await result.current.toggleHardwareArchiveAtom(!initialEnabled);
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings.hardwareArchive.enabled).toBe(
      initialEnabled,
    );
  });

  it("setHardwareArchiveRefreshIntervalDays: refreshIntervalDays is updated on success", async () => {
    (commands.setHardwareArchiveInterval as Mock).mockResolvedValue({
      data: null,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.setHardwareArchiveRefreshIntervalDays(7);
    });
    expect(result.current.settings.hardwareArchive.refreshIntervalDays).toBe(7);
  });

  it("setHardwareArchiveRefreshIntervalDays: error() is called on error and refreshIntervalDays is not updated", async () => {
    const errorMsg = "Failed to set archive interval";
    (commands.setHardwareArchiveInterval as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    const initialDays =
      result.current.settings.hardwareArchive.refreshIntervalDays;
    await act(async () => {
      await result.current.setHardwareArchiveRefreshIntervalDays(7);
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings.hardwareArchive.refreshIntervalDays).toBe(
      initialDays,
    );
  });

  it("setScheduledDataDeletion: scheduledDataDeletion is updated on success", async () => {
    (
      commands.setHardwareArchiveScheduledDataDeletion as Mock
    ).mockResolvedValue({ data: null });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    await act(async () => {
      await result.current.setScheduledDataDeletion(false);
    });
    expect(result.current.settings.hardwareArchive.scheduledDataDeletion).toBe(
      false,
    );
  });

  it("setScheduledDataDeletion: error() is called on error and scheduledDataDeletion is not updated", async () => {
    const errorMsg = "Failed to set scheduled deletion";
    (
      commands.setHardwareArchiveScheduledDataDeletion as Mock
    ).mockResolvedValue({ status: "error", error: errorMsg });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    const initialValue =
      result.current.settings.hardwareArchive.scheduledDataDeletion;
    await act(async () => {
      await result.current.setScheduledDataDeletion(!initialValue);
    });
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings.hardwareArchive.scheduledDataDeletion).toBe(
      initialValue,
    );
  });
});
