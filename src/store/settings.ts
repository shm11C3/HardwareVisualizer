import { atom, type Store } from "jotai";
import { defaultColorRGB } from "@/consts/chart";
import type { ClientSettings } from "@/rspc/bindings";

export const settingsAtom = atom<ClientSettings>({
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
  nsisMigrationNoticeDismissed: false,
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

type PowerDisplayTargets = ClientSettings["powerDisplayTargets"];

/**
 * Coalescing state for Power Display Target toggles.
 *
 * Rapid toggles apply optimistically to `desired`, while one in-flight loop
 * persists the latest `desired` value and records it as `persisted`. It is
 * owned per Jotai store so independent stores (tests, future windows) never
 * share an in-flight write.
 */
export type PowerDisplayTargetMutationState = {
  desired: PowerDisplayTargets | null;
  persisted: PowerDisplayTargets | null;
  inFlight: Promise<boolean> | null;
};

const powerDisplayTargetMutations = new WeakMap<
  Store,
  PowerDisplayTargetMutationState
>();

export const getPowerDisplayTargetMutationState = (
  store: Store,
): PowerDisplayTargetMutationState => {
  let state = powerDisplayTargetMutations.get(store);
  if (state === undefined) {
    state = { desired: null, persisted: null, inFlight: null };
    powerDisplayTargetMutations.set(store, state);
  }
  return state;
};
