import type { ClientSettings_Serialize } from "@/rspc/bindings";

/**
 * Deterministic settings returned by the mocked `get_settings` command.
 *
 * - `language: "en"` keeps captured text stable across machines.
 * - `closeToTrayChoiceMade: true` suppresses the first-run tray dialog so it
 *   never overlaps capture screenshots.
 * - `selectedBackgroundImg: null` avoids loading a background image.
 */
export const settingsFixture: ClientSettings_Serialize = {
  version: "1.0.0",
  language: "en",
  theme: "dark",
  displayTargets: ["cpu", "memory", "gpu"],
  graphSize: "xl",
  graphFitToWindow: false,
  graphMarginPx: 32,
  lineGraphType: "default",
  lineGraphBorder: true,
  lineGraphFill: true,
  // Comma-joined RGB components, matching the backend serialization
  // (settings_service.rs joins [u8;3] with ","). Consumers wrap the value
  // themselves: `rgb(${settings.lineGraphColor.cpu})` (LineChart.tsx).
  lineGraphColor: {
    cpu: "75,192,192",
    memory: "255,99,132",
    gpu: "255,206,86",
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
  burnInShift: false,
  burnInShiftMode: "jump",
  burnInShiftPreset: "aggressive",
  burnInShiftIdleOnly: false,
  burnInShiftOptions: null,
  textSelectable: false,
  closeToTray: false,
  closeToTrayChoiceMade: true,
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
};
