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
  navigationLayout: "grouped",
  uiAnnouncementVersion: 0,
  currentUiAnnouncementVersion: 0,
  displayTargets: [],
  powerDisplayTargets: ["cpu", "gpu", "package"],
  graphSize: "xl",
  graphFitToWindow: false,
  graphMarginPx: 32,
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
  glassBlur: 10,
  temperatureUnit: "C",
  hardwareArchive: {
    enabled: true,
    scheduledDataDeletion: true,
    retentionDays: 30,
  },
  storageHealth: {
    enabled: true,
    retentionDays: 1095,
  },
  environmentalSensors: {
    switchbotMeterEnabled: false,
  },
  burnInShift: false,
  burnInShiftPreset: "aggressive",
  burnInShiftMode: "jump",
  burnInShiftIdleOnly: false,
  burnInShiftOptions: null,
  textSelectable: false,
  closeToTray: false,
  closeToTrayChoiceMade: false,
  externalComponentGuidance: {
    acknowledgedKeys: [],
  },
  elevatedStartupMode: false,
  trayWidget: {
    enabled: false,
    metricOrder: ["cpu", "gpu", "gpu-temp"],
    visibleMetrics: ["cpu", "gpu", "gpu-temp"],
    updateIntervalSecs: 1,
  },
});

export const navigationMutationPendingAtom = atom(false);
let navigationMutationInFlight = false;
type PowerDisplayTargets = ClientSettings["powerDisplayTargets"];
let desiredPowerDisplayTargets: PowerDisplayTargets | null = null;
let persistedPowerDisplayTargets: PowerDisplayTargets | null = null;
let powerDisplayTargetMutation: Promise<boolean> | null = null;

const samePowerDisplayTargets = (
  left: PowerDisplayTargets,
  right: PowerDisplayTargets,
) =>
  left.length === right.length &&
  left.every((value, index) => value === right[index]);

export const useSettingsAtom = () => {
  const { error } = useTauriDialog();
  const mapSettingUpdater: {
    [K in keyof Omit<
      ClientSettings,
      | "state"
      | "lineGraphColor"
      | "version"
      | "hardwareArchive"
      | "storageHealth"
      | "environmentalSensors"
      | "closeToTray"
      | "closeToTrayChoiceMade"
      | "externalComponentGuidance"
      | "navigationLayout"
      | "uiAnnouncementVersion"
      | "currentUiAnnouncementVersion"
      | "trayWidget"
    >]: (value: ClientSettings[K]) => Promise<Result<null, string>>;
  } = {
    theme: commands.setTheme,
    displayTargets: commands.setDisplayTargets,
    powerDisplayTargets: commands.setPowerDisplayTargets,
    graphSize: commands.setGraphSize,
    graphFitToWindow: commands.setGraphFitToWindow,
    graphMarginPx: commands.setGraphMarginPx,
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
    glassBlur: commands.setGlassBlur,
    temperatureUnit: commands.setTemperatureUnit,
    burnInShift: commands.setBurnInShift,
    burnInShiftPreset: commands.setBurnInShiftPreset,
    burnInShiftMode: commands.setBurnInShiftMode,
    burnInShiftIdleOnly: commands.setBurnInShiftIdleOnly,
    burnInShiftOptions: commands.setBurnInShiftOptions,
    textSelectable: commands.setTextSelectable,
    elevatedStartupMode: commands.setElevatedStartupMode,
  };

  const [settings, setSettings] = useAtom(settingsAtom);
  const [, setNavigationMutationPending] = useAtom(
    navigationMutationPendingAtom,
  );

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
      | "storageHealth"
      | "environmentalSensors"
      | "closeToTray"
      | "closeToTrayChoiceMade"
      | "externalComponentGuidance"
      | "navigationLayout"
      | "uiAnnouncementVersion"
      | "currentUiAnnouncementVersion"
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

  const togglePowerDisplayTarget = async (
    target: ClientSettings["powerDisplayTargets"][number],
  ) => {
    if (desiredPowerDisplayTargets === null) {
      desiredPowerDisplayTargets = [...settings.powerDisplayTargets];
      persistedPowerDisplayTargets = [...settings.powerDisplayTargets];
    }

    desiredPowerDisplayTargets = desiredPowerDisplayTargets.includes(target)
      ? desiredPowerDisplayTargets.filter((value) => value !== target)
      : [...desiredPowerDisplayTargets, target];
    setSettings((prev) => ({
      ...prev,
      powerDisplayTargets:
        desiredPowerDisplayTargets ?? prev.powerDisplayTargets,
    }));

    if (powerDisplayTargetMutation === null) {
      powerDisplayTargetMutation = (async () => {
        while (
          desiredPowerDisplayTargets !== null &&
          persistedPowerDisplayTargets !== null &&
          !samePowerDisplayTargets(
            desiredPowerDisplayTargets,
            persistedPowerDisplayTargets,
          )
        ) {
          const nextTargets = [...desiredPowerDisplayTargets];
          const result = await commands.setPowerDisplayTargets(nextTargets);
          if (isError(result)) {
            await error(result.error);
            console.error(result.error);
            const rollbackTargets = persistedPowerDisplayTargets;
            setSettings((prev) => ({
              ...prev,
              powerDisplayTargets: rollbackTargets,
            }));
            desiredPowerDisplayTargets = null;
            persistedPowerDisplayTargets = null;
            powerDisplayTargetMutation = null;
            return false;
          }
          persistedPowerDisplayTargets = nextTargets;
        }

        desiredPowerDisplayTargets = null;
        persistedPowerDisplayTargets = null;
        powerDisplayTargetMutation = null;
        return true;
      })();
    }

    return powerDisplayTargetMutation;
  };

  const setNavigationLayoutAtom = async (
    value: ClientSettings["navigationLayout"],
  ) => {
    if (navigationMutationInFlight) return false;

    navigationMutationInFlight = true;
    setNavigationMutationPending(true);
    const previousLayout = settings.navigationLayout;
    const previousAnnouncementVersion = settings.uiAnnouncementVersion;
    const announcementVersion =
      value === "classic"
        ? Math.max(
            previousAnnouncementVersion,
            settings.currentUiAnnouncementVersion,
          )
        : previousAnnouncementVersion;

    setSettings((prev) => ({
      ...prev,
      navigationLayout: value,
      uiAnnouncementVersion: announcementVersion,
    }));

    try {
      const result = await commands.setNavigationLayout(value);

      if (isError(result)) {
        await error(result.error);
        console.error(result.error);
        setSettings((prev) => ({
          ...prev,
          navigationLayout: previousLayout,
          uiAnnouncementVersion: previousAnnouncementVersion,
        }));
        return false;
      }

      return true;
    } finally {
      navigationMutationInFlight = false;
      setNavigationMutationPending(false);
    }
  };

  const acknowledgeNavigationRestructureAnnouncementAtom = async () => {
    if (navigationMutationInFlight) return false;

    navigationMutationInFlight = true;
    setNavigationMutationPending(true);
    const previousValue = settings.uiAnnouncementVersion;
    setSettings((prev) => ({
      ...prev,
      uiAnnouncementVersion: Math.max(
        prev.uiAnnouncementVersion,
        prev.currentUiAnnouncementVersion,
      ),
    }));

    try {
      const result =
        await commands.acknowledgeNavigationRestructureAnnouncement();

      if (isError(result)) {
        await error(result.error);
        console.error(result.error);
        setSettings((prev) => ({
          ...prev,
          uiAnnouncementVersion: previousValue,
        }));
        return false;
      }

      return true;
    } finally {
      navigationMutationInFlight = false;
      setNavigationMutationPending(false);
    }
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

  /**
   * Returns whether the preference was actually persisted, so the caller
   * can tell a real change from a failed write. A refused write (a
   * corrupted settings.json, a read-only directory) leaves the scan
   * exactly as it was, and the settings screen must not follow it with a
   * "restart to apply" prompt for a change that did not happen.
   */
  const toggleSwitchbotMeterAtom = async (value: boolean) => {
    const result = await commands.setSwitchbotMeterEnabled(value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return false;
    }

    setSettings((prev) => ({
      ...prev,
      environmentalSensors: {
        ...prev.environmentalSensors,
        switchbotMeterEnabled: value,
        // Turning the source off clears the chosen device in Core, so
        // the screen must forget it too or the picker would keep
        // showing a selection the app no longer holds.
        switchbotMeterDevice: value
          ? (prev.environmentalSensors.switchbotMeterDevice ?? null)
          : null,
      },
    }));
    return true;
  };

  const setSwitchbotMeterDevice = async (deviceId: string) => {
    const result = await commands.setSwitchbotMeterDevice(deviceId);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return false;
    }

    setSettings((prev) => ({
      ...prev,
      environmentalSensors: {
        ...prev.environmentalSensors,
        switchbotMeterDevice: deviceId,
      },
    }));
    return true;
  };

  const setHardwareArchiveRetentionDays = async (value: number) => {
    const result = await commands.setHardwareArchiveRetentionDays(value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return;
    }

    setSettings((prev) => ({
      ...prev,
      hardwareArchive: { ...prev.hardwareArchive, retentionDays: value },
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

  const setStorageHealthRetentionDays = async (value: number) => {
    const result = await commands.setStorageHealthRetentionDays(value);

    if (isError(result)) {
      error(result.error);
      console.error(result.error);
      return false;
    }

    setSettings((prev) => ({
      ...prev,
      storageHealth: { ...prev.storageHealth, retentionDays: value },
    }));
    return true;
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
    togglePowerDisplayTarget,
    updateSettingAtom,
    updateLineGraphColorAtom,
    toggleHardwareArchiveAtom,
    toggleSwitchbotMeterAtom,
    setSwitchbotMeterDevice,
    setHardwareArchiveRetentionDays,
    setScheduledDataDeletion,
    setStorageHealthRetentionDays,
    setCloseToTrayPreferenceAtom,
    setTrayWidgetSettingsAtom,
    setNavigationLayoutAtom,
    acknowledgeNavigationRestructureAnnouncementAtom,
  };
};
