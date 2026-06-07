import { emit } from "@tauri-apps/api/event";
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { HardwareMonitorUpdate } from "@/rspc/bindings";
import { buildArchiveRecords, buildProcessStats } from "../fixtures/archive";
import {
  buildHardwareUpdateSeries,
  GPU_FIXTURES,
  processListFixture,
  storageHealthFixture,
  sysInfoFixture,
} from "../fixtures/hardware";
import { settingsFixture } from "../fixtures/settings";
import { storeFixture } from "../fixtures/store";

declare global {
  interface Window {
    __E2E__?: {
      /** Emit a single `hardware-monitor-update` event (fixture payload by default). */
      emitHardwareUpdate: (payload?: HardwareMonitorUpdate) => Promise<void>;
      /** Emit a deterministic series of updates so charts build up history. */
      emitHardwareUpdateSeries: (count?: number) => Promise<void>;
    };
  }
}

type InvokeHandler = (args?: unknown) => unknown;

const STORE_RID = 1;

const storeKey = (args?: unknown) => (args as { key: string }).key;

/**
 * Dispatch table mapping invoke commands to their mocked handlers:
 * Tauri plugin commands (`plugin:<name>|<command>`) and generated
 * tauri-specta commands (raw or typedError-boxed by the bindings).
 */
const buildInvokeHandlers = (
  store: Map<string, unknown>,
): Record<string, InvokeHandler> => ({
  // --- @tauri-apps/plugin-store (rid-based key-value store) ---
  "plugin:store|load": () => STORE_RID,
  "plugin:store|has": (args) => store.has(storeKey(args)),
  "plugin:store|get": (args) => [
    store.get(storeKey(args)) ?? null,
    store.has(storeKey(args)),
  ],
  "plugin:store|set": (args) => {
    const a = args as { key: string; value?: unknown };
    store.set(a.key, a.value);
    return null;
  },
  "plugin:store|delete": (args) => store.delete(storeKey(args)),
  "plugin:store|keys": () => [...store.keys()],
  "plugin:store|values": () => [...store.values()],
  "plugin:store|entries": () => [...store.entries()],
  "plugin:store|length": () => store.size,
  "plugin:store|save": () => null,
  "plugin:store|reload": () => null,
  "plugin:store|clear": () => null,
  "plugin:store|reset": () => null,

  // --- window/plugin surface ---
  "plugin:window|theme": () => "dark",
  "plugin:dialog|message": () => null,
  "plugin:autostart|is_enabled": () => false,
  "plugin:app|version": () => "1.0.0",

  // --- generated commands ---
  get_settings: () => settingsFixture,
  get_hardware_info: () => sysInfoFixture,
  get_process_list: () => processListFixture,
  get_storage_health_latest_records: () => storageHealthFixture,
  get_background_images: () => [],
  get_network_info: () => [],
  // No pending update — keeps the updater UI out of captures.
  fetch_update: () => null,
  // true keeps the settings capture free of the tray-unavailable warning,
  // matching a typical desktop session.
  is_close_to_tray_available: () => true,
  mark_close_to_tray_listener_ready: () => null,

  // --- insights archive commands (synthesized from requested range) ---
  get_gpu_archive_names: () => GPU_FIXTURES.map((gpu) => gpu.name),
  get_data_archive_records: (args) => {
    const a = args as { hardwareType: string; start: string; end: string };
    return a.hardwareType === "cpu"
      ? buildArchiveRecords(a.start, a.end, 45, 20)
      : buildArchiveRecords(a.start, a.end, 60, 8);
  },
  get_gpu_archive_records: (args) => {
    const a = args as { dataType: string; start: string; end: string };
    if (a.dataType === "temp") {
      return buildArchiveRecords(a.start, a.end, 58, 6);
    }
    if (a.dataType === "dedicatedMemory") {
      return buildArchiveRecords(a.start, a.end, 4_194_304, 524_288);
    }
    return buildArchiveRecords(a.start, a.end, 55, 25);
  },
  get_process_stats: (args) =>
    buildProcessStats((args as { endAt: string }).endAt),
  get_process_stats_in_period: (args) =>
    buildProcessStats((args as { end: string }).end),
});

/**
 * Install Tauri IPC/event/window mocks so the React app runs in a plain
 * browser with deterministic fixture data. Loaded from `src/main.tsx` only
 * when `VITE_E2E_MOCK=true` (the branch is dead-code eliminated otherwise).
 *
 * Mock layers:
 * - `mockWindows("main")` fakes the current window label.
 * - `__TAURI_OS_PLUGIN_INTERNALS__` backs the synchronous `platform()` API
 *   of @tauri-apps/plugin-os (not an IPC call).
 * - `mockIPC(..., { shouldMockEvents: true })` routes commands to the
 *   dispatch table above and implements `plugin:event|listen/emit/unlisten`.
 */
export const installTauriMocks = () => {
  mockWindows("main");

  (
    window as Window & { __TAURI_OS_PLUGIN_INTERNALS__?: unknown }
  ).__TAURI_OS_PLUGIN_INTERNALS__ = {
    platform: "windows",
    version: "10.0.26100",
    family: "windows",
    os_type: "windows",
    arch: "x86_64",
    exe_extension: "exe",
    eol: "\r\n",
  };

  const store = new Map<string, unknown>(Object.entries(storeFixture));
  const handlers = buildInvokeHandlers(store);

  mockIPC(
    (cmd: string, args?: unknown) => {
      if (Object.hasOwn(handlers, cmd)) {
        return handlers[cmd](args);
      }

      // Settings mutators (`set_theme`, `set_language`, ...) succeed silently
      // so settings-screen scenarios can interact without enumerating them.
      if (cmd.startsWith("set_")) {
        return null;
      }

      throw new Error(`[e2e-mock] Unhandled invoke: ${cmd}`);
    },
    { shouldMockEvents: true },
  );

  window.__E2E__ = {
    emitHardwareUpdate: (payload) =>
      emit(
        "hardware-monitor-update",
        payload ?? buildHardwareUpdateSeries(1)[0],
      ),
    emitHardwareUpdateSeries: async (count = 30) => {
      for (const payload of buildHardwareUpdateSeries(count)) {
        await emit("hardware-monitor-update", payload);
      }
    },
  };
};
