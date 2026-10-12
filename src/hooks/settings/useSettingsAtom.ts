import { useAtom, useStore } from "jotai";
import { useCallback } from "react";
import { defaultColorRGB } from "@/consts/chart";
import { RGB2HEX } from "@/lib/color";
import {
  type ClientSettings,
  commands,
  type LineGraphColorStringSettings,
} from "@/rspc/bindings";
import {
  getPowerDisplayTargetMutationState,
  navigationMutationPendingAtom,
  type SettingWriteAction,
  settingsAtom,
  settingsWriteFailuresAtom,
} from "@/store/settings";
import type { ChartDataType } from "@/types/chart";
import { chartHardwareTypes } from "@/types/chart";
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
  const store = useStore();

  /**
   * Every Application Preference write goes through one of two shapes, so a
   * failure is recorded or cleared exactly once per write by construction:
   *
   * - `persistOptimistic`: apply the new value first, then write, and roll
   *   back when the write fails (toggles and inputs that must feel instant).
   * - `persistThenApply`: write first and apply the new value only once it is
   *   saved (the value comes from, or must match, what the backend kept).
   *
   * A failed write is state, not a dialog: the control that owns `action`
   * renders it (`useSettingWriteFailure`). The backend detail only goes to
   * the log and the state. Failures are held per action, and only a later
   * successful write of the same action clears its own entry. Do not call
   * `recordWriteFailure` / `clearWriteFailure` from a mutator directly.
   */
  const recordWriteFailure = (action: SettingWriteAction, reason: unknown) => {
    console.error(`Failed to save setting "${action}":`, reason);
    store.set(settingsWriteFailuresAtom, (current) =>
      new Map(current).set(
        action,
        reason instanceof Error ? reason.message : String(reason),
      ),
    );
  };
  const clearWriteFailure = (action: SettingWriteAction) => {
    store.set(settingsWriteFailuresAtom, (current) => {
      if (!current.has(action)) return current;
      const next = new Map(current);
      next.delete(action);
      return next;
    });
  };

  /**
   * Optimistic write: `apply()` runs before `write()`. A backend error
   * records the failure, runs `rollback()` and resolves `false`. A rejected
   * call never reached the backend, so it is recorded and rolled back too,
   * then rethrown for the caller. Success clears the failure and resolves
   * `true`.
   */
  const persistOptimistic = async ({
    action,
    apply,
    rollback,
    write,
  }: {
    action: SettingWriteAction;
    apply: () => void;
    rollback: () => void;
    write: () => Promise<Result<unknown, string>>;
  }): Promise<boolean> => {
    apply();
    let result: Result<unknown, string>;
    try {
      result = await write();
    } catch (err) {
      recordWriteFailure(action, err);
      rollback();
      throw err;
    }

    if (isError(result)) {
      recordWriteFailure(action, result.error);
      rollback();
      return false;
    }

    clearWriteFailure(action);
    return true;
  };

  /**
   * Write-then-apply: `apply(data)` runs only after the backend accepted the
   * write, so a failure leaves the previous value untouched. A backend error
   * records the failure and resolves `false`. A rejected call is recorded and
   * rethrown (nothing was applied, so there is nothing to roll back); the
   * existing mutators of this shape never caught it either, because
   * `commands.*` reports failures as a `Result` and does not throw.
   */
  const persistThenApply = async <T>({
    action,
    write,
    apply,
  }: {
    action: SettingWriteAction;
    write: () => Promise<Result<T, string>>;
    apply: (data: T) => void;
  }): Promise<boolean> => {
    let result: Result<T, string>;
    try {
      result = await write();
    } catch (err) {
      recordWriteFailure(action, err);
      throw err;
    }

    if (isError(result)) {
      recordWriteFailure(action, result.error);
      return false;
    }

    clearWriteFailure(action);
    apply(result.data);
    return true;
  };

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

  const loadSettings = useCallback(async () => {
    try {
      const setting = await commands.getSettings();

      if (isError(setting)) {
        console.error("Failed to fetch settings:", setting.error);
        return false;
      }

      setSettings(setting.data);
      return true;
    } catch (err) {
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
    action: SettingWriteAction = key,
  ) => {
    const previousValue = settings[key];

    return persistOptimistic({
      action,
      apply: () => setSettings((prev) => ({ ...prev, [key]: value })),
      rollback: () =>
        setSettings((prev) => ({ ...prev, [key]: previousValue })),
      write: () => mapSettingUpdater[key](value),
    });
  };

  const toggleDisplayTarget = async (target: ChartDataType) => {
    const newTargets = settings.displayTargets.includes(target)
      ? settings.displayTargets.filter((t) => t !== target)
      : [...settings.displayTargets, target];

    await persistThenApply({
      action: "displayTargets",
      write: () => commands.setDisplayTargets(newTargets),
      apply: () =>
        setSettings((prev) => ({ ...prev, displayTargets: newTargets })),
    });
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
          const rollbackTargets = mutation.persisted;
          // The toggles already applied `desired` optimistically; each pass
          // only persists the latest value and settles its outcome.
          const saved = await persistOptimistic({
            action: "powerDisplayTargets",
            apply: () => {},
            rollback: () => {
              setSettings((prev) => ({
                ...prev,
                powerDisplayTargets: rollbackTargets,
              }));
              mutation.desired = null;
              mutation.persisted = null;
              mutation.inFlight = null;
            },
            write: () => commands.setPowerDisplayTargets(nextTargets),
          });
          if (!saved) {
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

    try {
      return await persistOptimistic({
        action: "navigationLayout",
        apply: () =>
          setSettings((prev) => ({
            ...prev,
            navigationLayout: value,
            uiAnnouncementVersion: announcementVersion,
          })),
        rollback: () =>
          setSettings((prev) => ({
            ...prev,
            navigationLayout: previousLayout,
            uiAnnouncementVersion: previousAnnouncementVersion,
          })),
        write: () => commands.setNavigationLayout(value),
      });
    } finally {
      store.set(navigationMutationPendingAtom, false);
    }
  };

  const acknowledgeNavigationRestructureAnnouncementAtom = async () => {
    if (store.get(navigationMutationPendingAtom)) return false;

    store.set(navigationMutationPendingAtom, true);
    const previousValue = settings.uiAnnouncementVersion;

    try {
      return await persistOptimistic({
        action: "uiAnnouncementVersion",
        apply: () =>
          setSettings((prev) => ({
            ...prev,
            uiAnnouncementVersion: Math.max(
              prev.uiAnnouncementVersion,
              prev.currentUiAnnouncementVersion,
            ),
          })),
        rollback: () =>
          setSettings((prev) => ({
            ...prev,
            uiAnnouncementVersion: previousValue,
          })),
        write: () => commands.acknowledgeNavigationRestructureAnnouncement(),
      });
    } finally {
      store.set(navigationMutationPendingAtom, false);
    }
  };

  /** Applies a color the backend returned (it normalizes the code). */
  const applyLineGraphColor = (
    key: keyof LineGraphColorStringSettings,
    color: string,
  ) =>
    setSettings((prev) => ({
      ...prev,
      lineGraphColor: { ...prev.lineGraphColor, [key]: color },
    }));

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
    await persistThenApply({
      action: `lineGraphColor.${key}`,
      write: () => commands.setLineGraphColor(key, value),
      apply: (color) => applyLineGraphColor(key, color),
    });
  };

  /**
   * Restores every line graph color to its default as one operation. Each
   * color keeps its own outcome (a failed write leaves that color as it was),
   * and `lineGraphColor.reset` is recorded once from the aggregate: a failure
   * if any write failed, cleared only if all succeeded.
   *
   * @returns whether every color was restored
   */
  const resetLineGraphColors = async () => {
    const results = await Promise.all(
      chartHardwareTypes.map(async (type) => {
        const result = await commands.setLineGraphColor(
          type,
          RGB2HEX(defaultColorRGB[type]),
        );
        if (!isError(result)) {
          applyLineGraphColor(type, result.data);
        }
        return result;
      }),
    );
    const failed = results.find(isError);

    if (failed !== undefined) {
      recordWriteFailure("lineGraphColor.reset", failed.error);
      return false;
    }

    clearWriteFailure("lineGraphColor.reset");
    return true;
  };

  const toggleHardwareArchiveAtom = async (value: boolean) => {
    await persistThenApply({
      action: "hardwareArchive.enabled",
      write: () => commands.setHardwareArchiveEnabled(value),
      apply: () =>
        setSettings((prev) => ({
          ...prev,
          hardwareArchive: { ...prev.hardwareArchive, enabled: value },
        })),
    });
  };

  /**
   * Returns whether the preference was actually persisted, so the caller
   * can tell a real change from a failed write. A refused write (a
   * corrupted settings.json, a read-only directory) leaves the scan
   * exactly as it was, and the settings screen must not follow it with a
   * "restart to apply" prompt for a change that did not happen.
   */
  const toggleSwitchbotMeterAtom = (value: boolean) =>
    persistThenApply({
      action: "environmentalSensors.enabled",
      write: () => commands.setSwitchbotMeterEnabled(value),
      apply: () =>
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
        })),
    });

  const setSwitchbotMeterDevice = (deviceId: string) =>
    persistThenApply({
      action: "environmentalSensors.device",
      write: () => commands.setSwitchbotMeterDevice(deviceId),
      apply: () =>
        setSettings((prev) => ({
          ...prev,
          environmentalSensors: {
            ...prev.environmentalSensors,
            switchbotMeterDevice: deviceId,
          },
        })),
    });

  /**
   * `action` names the control that asked, so the retention input and the
   * database conversion notice each show only their own failure.
   */
  const setHardwareArchiveRetentionDays = (
    value: number,
    action: SettingWriteAction = "hardwareArchive.retentionDays",
  ) =>
    persistThenApply({
      action,
      write: () => commands.setHardwareArchiveRetentionDays(value),
      apply: () =>
        setSettings((prev) => ({
          ...prev,
          hardwareArchive: { ...prev.hardwareArchive, retentionDays: value },
        })),
    });

  const setScheduledDataDeletion = async (value: boolean) => {
    await persistThenApply({
      action: "hardwareArchive.scheduledDeletion",
      write: () => commands.setHardwareArchiveScheduledDataDeletion(value),
      apply: () =>
        setSettings((prev) => ({
          ...prev,
          hardwareArchive: {
            ...prev.hardwareArchive,
            scheduledDataDeletion: value,
          },
        })),
    });
  };

  const setStorageHealthRetentionDays = (value: number) =>
    persistThenApply({
      action: "storageHealth.retentionDays",
      write: () => commands.setStorageHealthRetentionDays(value),
      apply: () =>
        setSettings((prev) => ({
          ...prev,
          storageHealth: { ...prev.storageHealth, retentionDays: value },
        })),
    });

  const setCloseToTrayPreferenceAtom = (value: boolean) => {
    const previousCloseToTray = settings.closeToTray;
    const previousChoiceMade = settings.closeToTrayChoiceMade;
    const previousTrayWidget = settings.trayWidget;
    const shouldEnableTrayWidget = value && !previousTrayWidget.enabled;
    let trayWidgetPersisted = false;
    // Set when undoing the tray widget save failed: the backend then still
    // has the widget enabled, so the restored atom must keep it enabled too.
    let keepTrayWidgetEnabled = false;

    // Two backend writes, one outcome: `closeToTray` is recorded or cleared
    // once, by `persistOptimistic`, from whatever `write` resolves to.
    return persistOptimistic({
      action: "closeToTray",
      apply: () =>
        setSettings((prev) => ({
          ...prev,
          closeToTray: value,
          closeToTrayChoiceMade: true,
          trayWidget: value
            ? { ...prev.trayWidget, enabled: true }
            : prev.trayWidget,
        })),
      rollback: () =>
        setSettings((prev) => ({
          ...prev,
          closeToTray: previousCloseToTray,
          closeToTrayChoiceMade: previousChoiceMade,
          ...(keepTrayWidgetEnabled ? {} : { trayWidget: previousTrayWidget }),
        })),
      write: async () => {
        try {
          if (shouldEnableTrayWidget) {
            const trayWidgetResult = await commands.setTrayWidgetSettings({
              ...previousTrayWidget,
              enabled: true,
            });
            if (isError(trayWidgetResult)) {
              return trayWidgetResult;
            }
            trayWidgetPersisted = true;
          }

          const result = await commands.setCloseToTrayPreference(value);

          if (isError(result) && shouldEnableTrayWidget) {
            const rollbackResult =
              await commands.setTrayWidgetSettings(previousTrayWidget);
            if (isError(rollbackResult)) {
              console.error(rollbackResult.error);
              keepTrayWidgetEnabled = true;
            }
          }

          return result;
        } catch (err) {
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
          throw err;
        }
      },
    });
  };

  const setTrayWidgetSettingsAtom = (value: ClientSettings["trayWidget"]) => {
    const previousValue = settings.trayWidget;

    return persistOptimistic({
      action: "trayWidget",
      apply: () => setSettings((prev) => ({ ...prev, trayWidget: value })),
      rollback: () =>
        setSettings((prev) => ({ ...prev, trayWidget: previousValue })),
      write: () => commands.setTrayWidgetSettings(value),
    });
  };

  return {
    settings,
    loadSettings,
    toggleDisplayTarget,
    togglePowerDisplayTarget,
    updateSettingAtom,
    updateLineGraphColorAtom,
    resetLineGraphColors,
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
