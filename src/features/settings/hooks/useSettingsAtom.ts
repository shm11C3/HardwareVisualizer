import { atom, useAtom } from "jotai";
import { useCallback } from "react";
import { defaultColorRGB } from "@/features/hardware/consts/chart";
import type { ChartDataType } from "@/features/hardware/types/hardwareDataType";
import type { Settings } from "@/features/settings/types/settingsType";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type ClientSettings, commands } from "@/rspc/bindings";
import type { Result } from "@/types/result";
import { isError } from "@/types/result";

const settingsAtom = atom<ClientSettings>({
  version: "0.0.0",
  language: "en",
  theme: "system",
  displayTargets: [],
  graphSize: "xl",
  lineGraphType: "default",
  lineGraphBorder: true,
  lineGraphFill: true,
  lineGraphColor: {
    cpu: `rgb(${defaultColorRGB.cpu})`,
    memory: `rgb(${defaultColorRGB.memory})`,
    gpu: `rgb(${defaultColorRGB.gpu})`,
  },
  lineGraphMix: true,
  lineGraphShowLegend: true,
  lineGraphShowScale: false,
  lineGraphShowTooltip: true,
  backgroundImgOpacity: 50,
  selectedBackgroundImg: null,
  transparentUi: false,
  windowOpacity: 86,
  temperatureUnit: "C",
  hardwareArchive: {
    enabled: true,
    scheduledDataDeletion: true,
    refreshIntervalDays: 30,
  },
  burnInShift: false,
  burnInShiftPreset: "aggressive",
  burnInShiftMode: "jump",
  burnInShiftIdleOnly: false,
  burnInShiftOptions: null,
  textSelectable: false,
  closeToTray: false,
  closeToTrayChoiceMade: false,
  trayWidget: {
    enabled: false,
    metricOrder: ["cpu", "gpu", "gpu-temp"],
    visibleMetrics: ["cpu", "gpu", "gpu-temp"],
    updateIntervalSecs: 1,
  },
});

export const useSettingsAtom = () => {
  const { error } = useTauriDialog();
  const mapSettingUpdater: {
    [K in keyof Omit<
      ClientSettings,
      | "state"
      | "lineGraphColor"
      | "version"
      | "hardwareArchive"
      | "closeToTray"
      | "closeToTrayChoiceMade"
      | "trayWidget"
    >]: (value: ClientSettings[K]) => Promise<Result<null, string>>;
  } = {
    theme: commands.setTheme,
    displayTargets: commands.setDisplayTargets,
    graphSize: commands.setGraphSize,
    lineGraphType: commands.setLineGraphType,
    language: commands.setLanguage,
    lineGraphBorder: commands.setLineGraphBorder,
    lineGraphFill: commands.setLineGraphFill,
    lineGraphMix: commands.setLineGraphMix,
    lineGraphShowLegend: commands.setLineGraphShowLegend,
    lineGraphShowScale: commands.setLineGraphShowScale,
    lineGraphShowTooltip: commands.setLineGraphShowTooltip,
    backgroundImgOpacity: commands.setBackgroundImgOpacity,
    selectedBackgroundImg: commands.setSelectedBackgroundImg,
    transparentUi: commands.setTransparentUi,
    windowOpacity: commands.setWindowOpacity,
    temperatureUnit: commands.setTemperatureUnit,
    burnInShift: commands.setBurnInShift,
    burnInShiftPreset: commands.setBurnInShiftPreset,
    burnInShiftMode: commands.setBurnInShiftMode,
    burnInShiftIdleOnly: commands.setBurnInShiftIdleOnly,
    burnInShiftOptions: commands.setBurnInShiftOptions,
    textSelectable: commands.setTextSelectable,
  };

  const [settings, setSettings] = useAtom(settingsAtom);

  // biome-ignore lint/correctness/useExhaustiveDependencies: This effect runs only once to load settings
  const loadSettings = useCallback(async () => {
    try {
      const setting = await commands.getSettings();

      if (isError(setting)) {
        await error(setting.error);
        console.error("Failed to fetch settings:", setting.error);
        return false;
      }

      setSettings(setting.data);
      return true;
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      await error(message);
      console.error("Failed to fetch settings:", err);
      return false;
    }
  }, [setSettings]);

  const updateSettingAtom = async <
    K extends keyof Omit<
      ClientSettings,
      | "state"
      | "lineGraphColor"
      | "version"
      | "hardwareArchive"
      | "closeToTray"
      | "closeToTrayChoiceMade"
      | "trayWidget"
    >,
  >(
    key: K,
    value: ClientSettings[K],
  ) => {
    const previousValue = settings[key];

    setSettings((prev) => ({ ...prev, [key]: value }));
    const result = await mapSettingUpdater[key](value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      setSettings((prev) => ({ ...prev, [key]: previousValue }));
    }
  };

  const toggleDisplayTarget = async (target: ChartDataType) => {
    const newTargets = settings.displayTargets.includes(target)
      ? settings.displayTargets.filter((t) => t !== target)
      : [...settings.displayTargets, target];

    const result = await commands.setDisplayTargets(newTargets);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return;
    }

    setSettings((prev) => ({ ...prev, displayTargets: newTargets }));
  };

  /**
   * Update color code
   *
   * @param key
   * @param value Color code in hexadecimal format
   */
  const updateLineGraphColorAtom = async (
    key: keyof Settings["lineGraphColor"],
    value: string,
  ) => {
    const result = await commands.setLineGraphColor(key, value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return;
    }

    setSettings((prev) => ({
      ...prev,
      lineGraphColor: { ...prev.lineGraphColor, [key]: result.data },
    }));
  };

  const toggleHardwareArchiveAtom = async (value: boolean) => {
    const result = await commands.setHardwareArchiveEnabled(value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return;
    }

    setSettings((prev) => ({
      ...prev,
      hardwareArchive: { ...prev.hardwareArchive, enabled: value },
    }));
  };

  const setHardwareArchiveRefreshIntervalDays = async (value: number) => {
    const result = await commands.setHardwareArchiveInterval(value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return;
    }

    setSettings((prev) => ({
      ...prev,
      hardwareArchive: { ...prev.hardwareArchive, refreshIntervalDays: value },
    }));
  };

  const setScheduledDataDeletion = async (value: boolean) => {
    const result =
      await commands.setHardwareArchiveScheduledDataDeletion(value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return;
    }

    setSettings((prev) => ({
      ...prev,
      hardwareArchive: {
        ...prev.hardwareArchive,
        scheduledDataDeletion: value,
      },
    }));
  };

  const setCloseToTrayPreferenceAtom = async (value: boolean) => {
    const previousCloseToTray = settings.closeToTray;
    const previousChoiceMade = settings.closeToTrayChoiceMade;
    const previousTrayWidget = settings.trayWidget;

    setSettings((prev) => ({
      ...prev,
      closeToTray: value,
      closeToTrayChoiceMade: true,
      trayWidget: value
        ? { ...prev.trayWidget, enabled: true }
        : prev.trayWidget,
    }));

    const shouldEnableTrayWidget = value && !previousTrayWidget.enabled;

    if (shouldEnableTrayWidget) {
      const nextTrayWidget = { ...previousTrayWidget, enabled: true };
      const trayWidgetResult =
        await commands.setTrayWidgetSettings(nextTrayWidget);

      if (isError(trayWidgetResult)) {
        error(trayWidgetResult.error);
        console.error(trayWidgetResult.error);
        setSettings((prev) => ({
          ...prev,
          closeToTray: previousCloseToTray,
          closeToTrayChoiceMade: previousChoiceMade,
          trayWidget: previousTrayWidget,
        }));
        return false;
      }
    }

    const result = await commands.setCloseToTrayPreference(value);

    if (isError(result)) {
      if (shouldEnableTrayWidget) {
        const rollbackResult =
          await commands.setTrayWidgetSettings(previousTrayWidget);

        if (isError(rollbackResult)) {
          await error(rollbackResult.error);
          console.error(rollbackResult.error);
          console.error(result.error);
          setSettings((prev) => ({
            ...prev,
            closeToTray: previousCloseToTray,
            closeToTrayChoiceMade: previousChoiceMade,
          }));
          return false;
        }
      }

      await error(result.error);
      console.error(result.error);
      setSettings((prev) => ({
        ...prev,
        closeToTray: previousCloseToTray,
        closeToTrayChoiceMade: previousChoiceMade,
        trayWidget: previousTrayWidget,
      }));
      return false;
    }

    return true;
  };

  const setTrayWidgetSettingsAtom = async (
    value: ClientSettings["trayWidget"],
  ) => {
    const previousValue = settings.trayWidget;

    setSettings((prev) => ({ ...prev, trayWidget: value }));
    const result = await commands.setTrayWidgetSettings(value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      setSettings((prev) => ({ ...prev, trayWidget: previousValue }));
      return false;
    }

    return true;
  };

  return {
    settings,
    loadSettings,
    toggleDisplayTarget,
    updateSettingAtom,
    updateLineGraphColorAtom,
    toggleHardwareArchiveAtom,
    setHardwareArchiveRefreshIntervalDays,
    setScheduledDataDeletion,
    setCloseToTrayPreferenceAtom,
    setTrayWidgetSettingsAtom,
  };
};
