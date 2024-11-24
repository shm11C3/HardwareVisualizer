import { defaultColorRGB } from "@/consts/chart";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type Result, commands } from "@/rspc/bindings";
import type { ChartDataType } from "@/types/hardwareDataType";
import { isError } from "@/types/result";
import type { Settings } from "@/types/settingsType";
import { atom, useAtom } from "jotai";
import { useCallback } from "react";

const settingsAtom = atom<Settings>({
  language: "en",
  theme: "light",
  displayTargets: [],
  graphSize: "xl",
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
  backgroundImgOpacity: 50,
  selectedBackgroundImg: null,
});

export const useSettingsAtom = () => {
  const { error } = useTauriDialog();
  const mapSettingUpdater: {
    [K in keyof Omit<Settings, "state" | "lineGraphColor">]: (
      value: Settings[K],
    ) => Promise<Result<null, string>>;
  } = {
    theme: commands.setTheme,
    displayTargets: commands.setDisplayTargets,
    graphSize: commands.setGraphSize,
    language: commands.setLanguage,
    lineGraphBorder: commands.setLineGraphBorder,
    lineGraphFill: commands.setLineGraphFill,
    lineGraphMix: commands.setLineGraphMix,
    lineGraphShowLegend: commands.setLineGraphShowLegend,
    lineGraphShowScale: commands.setLineGraphShowScale,
    backgroundImgOpacity: commands.setBackgroundImgOpacity,
    selectedBackgroundImg: commands.setSelectedBackgroundImg,
  };

  const [settings, setSettings] = useAtom(settingsAtom);

  // biome-ignore lint/correctness/useExhaustiveDependencies: <explanation>
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
    K extends keyof Omit<Settings, "state" | "lineGraphColor">,
  >(
    key: K,
    value: Settings[K],
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
      lineGraphColor: { ...prev.lineGraphColor, [key]: result },
    }));
  };

  return {
    settings,
    loadSettings,
    toggleDisplayTarget,
    updateSettingAtom,
    updateLineGraphColorAtom,
  };
};
