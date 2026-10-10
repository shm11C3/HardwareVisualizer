import { useAtom, useStore } from "jotai";
import { useCallback } from "react";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import {
  type ClientSettings,
  commands,
  type LineGraphColorStringSettings,
} from "@/rspc/bindings";
import {
  getPowerDisplayTargetMutationState,
  navigationMutationPendingAtom,
  settingsAtom,
} from "@/store/settings";
import type { ChartDataType } from "@/types/chart";
import type { Result } from "@/types/result";
import { isError } from "@/types/result";

type PowerDisplayTargets = ClientSettings["powerDisplayTargets"];

const samePowerDisplayTargets = (
  left: PowerDisplayTargets,
  right: PowerDisplayTargets,
) =>
  left.length === right.length &&
  left.every((value, index) => value === right[index]);

export const useSettingsAtom = () => {
  const { error } = useTauriDialog();
  const store = useStore();
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
      | "nsisMigrationNoticeDismissed"
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
      | "nsisMigrationNoticeDismissed"
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
    let result: Result<null, string>;
    try {
      result = await mapSettingUpdater[key](value);
    } catch (err) {
      // A rejected IPC call never reached the backend, so the optimistic
      // value must not stay on screen. The caller owns the user-facing
      // message, so rethrow instead of showing the raw error here.
      console.error(err);
      setSettings((prev) => ({ ...prev, [key]: previousValue }));
      throw err;
    }

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
    const mutation = getPowerDisplayTargetMutationState(store);
    if (mutation.desired === null) {
      mutation.desired = [...settings.powerDisplayTargets];
      mutation.persisted = [...settings.powerDisplayTargets];
    }

    mutation.desired = mutation.desired.includes(target)
      ? mutation.desired.filter((value) => value !== target)
      : [...mutation.desired, target];
    setSettings((prev) => ({
      ...prev,
      powerDisplayTargets: mutation.desired ?? prev.powerDisplayTargets,
    }));

    if (mutation.inFlight === null) {
      mutation.inFlight = (async () => {
        while (
          mutation.desired !== null &&
          mutation.persisted !== null &&
          !samePowerDisplayTargets(mutation.desired, mutation.persisted)
        ) {
          const nextTargets = [...mutation.desired];
          const result = await commands.setPowerDisplayTargets(nextTargets);
          if (isError(result)) {
            await error(result.error);
            console.error(result.error);
            const rollbackTargets = mutation.persisted;
            setSettings((prev) => ({
              ...prev,
              powerDisplayTargets: rollbackTargets,
            }));
            mutation.desired = null;
            mutation.persisted = null;
            mutation.inFlight = null;
            return false;
          }
          mutation.persisted = nextTargets;
        }

        mutation.desired = null;
        mutation.persisted = null;
        mutation.inFlight = null;
        return true;
      })();
    }

    return mutation.inFlight;
  };

  const setNavigationLayoutAtom = async (
    value: ClientSettings["navigationLayout"],
  ) => {
    if (store.get(navigationMutationPendingAtom)) return false;

    store.set(navigationMutationPendingAtom, true);
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
    } catch (err) {
      console.error(err);
      setSettings((prev) => ({
        ...prev,
        navigationLayout: previousLayout,
        uiAnnouncementVersion: previousAnnouncementVersion,
      }));
      throw err;
    } finally {
      store.set(navigationMutationPendingAtom, false);
    }
  };

  const acknowledgeNavigationRestructureAnnouncementAtom = async () => {
    if (store.get(navigationMutationPendingAtom)) return false;

    store.set(navigationMutationPendingAtom, true);
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
    } catch (err) {
      console.error(err);
      setSettings((prev) => ({
        ...prev,
        uiAnnouncementVersion: previousValue,
      }));
      throw err;
    } finally {
      store.set(navigationMutationPendingAtom, false);
    }
  };

  /**
   * Update color code
   *
   * @param key
   * @param value Color code in hexadecimal format
   */
  const updateLineGraphColorAtom = async (
    key: keyof LineGraphColorStringSettings,
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
      return false;
    }

    setSettings((prev) => ({
      ...prev,
      hardwareArchive: { ...prev.hardwareArchive, retentionDays: value },
    }));
    return true;
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
    let trayWidgetPersisted = false;

    try {
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
        trayWidgetPersisted = true;
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
    } catch (err) {
      console.error(err);
      if (trayWidgetPersisted) {
        // The widget save already landed on disk; undo it best-effort so
        // the backend matches the restored atom. Its own failure is only
        // logged so it never masks the original rejection.
        try {
          const undoResult =
            await commands.setTrayWidgetSettings(previousTrayWidget);
          if (isError(undoResult)) {
            console.error(undoResult.error);
          }
        } catch (undoErr) {
          console.error(undoErr);
        }
      }
      setSettings((prev) => ({
        ...prev,
        closeToTray: previousCloseToTray,
        closeToTrayChoiceMade: previousChoiceMade,
        trayWidget: previousTrayWidget,
      }));
      throw err;
    }
  };

  const setTrayWidgetSettingsAtom = async (
    value: ClientSettings["trayWidget"],
  ) => {
    const previousValue = settings.trayWidget;

    setSettings((prev) => ({ ...prev, trayWidget: value }));
    let result: Result<null, string>;
    try {
      result = await commands.setTrayWidgetSettings(value);
    } catch (err) {
      console.error(err);
      setSettings((prev) => ({ ...prev, trayWidget: previousValue }));
      throw err;
    }

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
