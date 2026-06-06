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
    setTransparentUi: vi.fn(),
    setWindowOpacity: vi.fn(),
    setTemperatureUnit: vi.fn(),
    setLineGraphColor: vi.fn(),
    setBurnInShift: vi.fn(),
    setBurnInShiftPreset: vi.fn(),
    setBurnInShiftMode: vi.fn(),
    setBurnInShiftIdleOnly: vi.fn(),
    setBurnInShiftOptions: vi.fn(),
    setCloseToTrayPreference: vi.fn(),
    setTrayWidgetSettings: vi.fn(),
    setHardwareArchiveEnabled: vi.fn(),
    setHardwareArchiveInterval: vi.fn(),
    setHardwareArchiveScheduledDataDeletion: vi.fn(),
    setStorageHealthRetentionDays: vi.fn(),
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
    let loaded = false;
    await act(async () => {
      loaded = await result.current.loadSettings();
    });
    expect(loaded).toBe(true);
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
    let loaded = true;
    await act(async () => {
      loaded = await result.current.loadSettings();
    });
    expect(loaded).toBe(false);
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

  it("setStorageHealthRetentionDays: retentionDays is updated on success", async () => {
    (commands.setStorageHealthRetentionDays as Mock).mockResolvedValue({
      data: null,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    let saved = false;
    await act(async () => {
      saved = await result.current.setStorageHealthRetentionDays(730);
    });
    expect(saved).toBe(true);
    expect(result.current.settings.storageHealth.retentionDays).toBe(730);
  });

  it("setStorageHealthRetentionDays: error() is called on error and retentionDays is not updated", async () => {
    const errorMsg = "Failed to set SMART retention";
    (commands.setStorageHealthRetentionDays as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });
    const initialDays = result.current.settings.storageHealth.retentionDays;
    let saved = true;
    await act(async () => {
      saved = await result.current.setStorageHealthRetentionDays(730);
    });
    expect(saved).toBe(false);
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings.storageHealth.retentionDays).toBe(
      initialDays,
    );
  });

  it("setCloseToTrayPreferenceAtom: enabling close-to-tray also enables the tray widget", async () => {
    (commands.setCloseToTrayPreference as Mock).mockResolvedValue({
      data: null,
    });
    (commands.setTrayWidgetSettings as Mock).mockResolvedValue({
      data: null,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });

    await act(async () => {
      await result.current.setCloseToTrayPreferenceAtom(true);
    });

    expect(commands.setCloseToTrayPreference).toHaveBeenCalledWith(true);
    expect(commands.setTrayWidgetSettings).toHaveBeenCalledWith({
      enabled: true,
      metricOrder: ["cpu", "gpu", "gpu-temp"],
      visibleMetrics: ["cpu", "gpu", "gpu-temp"],
      updateIntervalSecs: 1,
    });
    expect(result.current.settings.closeToTray).toBe(true);
    expect(result.current.settings.closeToTrayChoiceMade).toBe(true);
    expect(result.current.settings.trayWidget.enabled).toBe(true);
  });

  it("setCloseToTrayPreferenceAtom: widget save failure reverts the optimistic tray-mode update", async () => {
    const errorMsg = "Failed to enable tray widget";
    (commands.setCloseToTrayPreference as Mock).mockResolvedValue({
      data: null,
    });
    (commands.setTrayWidgetSettings as Mock).mockResolvedValue({
      status: "error",
      error: errorMsg,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });

    const initialSettings = result.current.settings;
    let saved = true;

    await act(async () => {
      saved = await result.current.setCloseToTrayPreferenceAtom(true);
    });

    expect(saved).toBe(false);
    expect(errorMock).toHaveBeenCalledWith(errorMsg);
    expect(result.current.settings.closeToTray).toBe(
      initialSettings.closeToTray,
    );
    expect(result.current.settings.closeToTrayChoiceMade).toBe(
      initialSettings.closeToTrayChoiceMade,
    );
    expect(result.current.settings.trayWidget).toEqual(
      initialSettings.trayWidget,
    );
  });

  it("setCloseToTrayPreferenceAtom: widget rollback failure keeps local widget state aligned with backend", async () => {
    const preferenceError = "Failed to save close-to-tray";
    const rollbackError = "Failed to roll back tray widget";
    (commands.setTrayWidgetSettings as Mock)
      .mockResolvedValueOnce({ data: null })
      .mockResolvedValueOnce({
        status: "error",
        error: rollbackError,
      });
    (commands.setCloseToTrayPreference as Mock).mockResolvedValue({
      status: "error",
      error: preferenceError,
    });

    const { result } = renderHook(() => useSettingsAtom(), {
      wrapper: Provider,
    });

    const initialSettings = result.current.settings;
    let saved = true;

    await act(async () => {
      saved = await result.current.setCloseToTrayPreferenceAtom(true);
    });

    expect(saved).toBe(false);
    expect(commands.setTrayWidgetSettings).toHaveBeenNthCalledWith(1, {
      ...initialSettings.trayWidget,
      enabled: true,
    });
    expect(commands.setTrayWidgetSettings).toHaveBeenNthCalledWith(
      2,
      initialSettings.trayWidget,
    );
    expect(errorMock).toHaveBeenCalledWith(rollbackError);
    expect(result.current.settings.closeToTray).toBe(
      initialSettings.closeToTray,
    );
    expect(result.current.settings.closeToTrayChoiceMade).toBe(
      initialSettings.closeToTrayChoiceMade,
    );
    expect(result.current.settings.trayWidget.enabled).toBe(true);
  });
});
