import { atom, useAtom } from "jotai";
import { useCallback } from "react";
import { defaultColorRGB } from "@/features/hardware/consts/chart";
import type { ChartDataType } from "@/features/hardware/types/hardwareDataType";
import type { Settings } from "@/features/settings/types/settingsType";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type ClientSettings, commands, type Result } from "@/rspc/bindings";
import { isError } from "@/types/result";

const settingsAtom = atom<ClientSettings>({
  version: "0.0.0",
  language: "en",
  theme: "light",
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
});

export const useSettingsAtom = () => {
  const { error } = useTauriDialog();
  const mapSettingUpdater: {
    [K in keyof Omit<
      ClientSettings,
      "state" | "lineGraphColor" | "version" | "hardwareArchive"
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
    temperatureUnit: commands.setTemperatureUnit,
    burnInShift: commands.setBurnInShift,
    burnInShiftPreset: commands.setBurnInShiftPreset,
    burnInShiftMode: commands.setBurnInShiftMode,
    burnInShiftIdleOnly: commands.setBurnInShiftIdleOnly,
  };

  const [settings, setSettings] = useAtom(settingsAtom);

  // biome-ignore lint/correctness/useExhaustiveDependencies: This effect runs only once to load settings
  const loadSettings = useCallback(async () => {
    const setting = await commands.getSettings();

    if (isError(setting)) {
      error(setting.error);
      console.error("Failed to fetch settings:", setting.error);
      return;
    }

    setSettings(setting.data);
  }, [setSettings]);

  const updateSettingAtom = async <
    K extends keyof Omit<
      ClientSettings,
      "state" | "lineGraphColor" | "version" | "hardwareArchive"
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
   * カラーコードを更新する
   *
   * @param key
   * @param value 16進数形式のカラーコード
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

  return {
    settings,
    loadSettings,
    toggleDisplayTarget,
    updateSettingAtom,
    updateLineGraphColorAtom,
    toggleHardwareArchiveAtom,
    setHardwareArchiveRefreshIntervalDays,
    setScheduledDataDeletion,
  };
};
