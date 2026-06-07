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

const STORE_RID = 1;

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
 *   handler below and implements `plugin:event|listen/emit/unlisten`.
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

  mockIPC(
    (cmd: string, args?: unknown) => {
      // --- @tauri-apps/plugin-store (rid-based key-value store) ---
      if (cmd.startsWith("plugin:store|")) {
        const a = args as { key?: string; value?: unknown };
        switch (cmd) {
          case "plugin:store|load":
            return STORE_RID;
          case "plugin:store|has":
            return store.has(a.key as string);
          case "plugin:store|get":
            return [
              store.get(a.key as string) ?? null,
              store.has(a.key as string),
            ];
          case "plugin:store|set":
            store.set(a.key as string, a.value);
            return null;
          case "plugin:store|delete":
            return store.delete(a.key as string);
          case "plugin:store|keys":
            return [...store.keys()];
          case "plugin:store|values":
            return [...store.values()];
          case "plugin:store|entries":
            return [...store.entries()];
          case "plugin:store|length":
            return store.size;
          case "plugin:store|save":
          case "plugin:store|reload":
          case "plugin:store|clear":
          case "plugin:store|reset":
            return null;
        }
      }

      switch (cmd) {
        // --- window/plugin surface ---
        case "plugin:window|theme":
          return "dark";
        case "plugin:dialog|message":
          return null;
        case "plugin:autostart|is_enabled":
          return false;
        case "plugin:app|version":
          return "1.0.0";

        // --- generated commands (raw or typedError-boxed by bindings) ---
        case "get_settings":
          return settingsFixture;
        case "get_hardware_info":
          return sysInfoFixture;
        case "get_process_list":
          return processListFixture;
        case "get_storage_health_latest_records":
          return storageHealthFixture;
        case "get_background_images":
          return [];
        case "get_network_info":
          return [];
        case "fetch_update":
          // No pending update — keeps the updater UI out of captures.
          return null;
        case "is_close_to_tray_available":
          // true keeps the settings capture free of the tray-unavailable
          // warning, matching a typical desktop session.
          return true;
        case "mark_close_to_tray_listener_ready":
          return null;

        // --- insights archive commands (synthesized from requested range) ---
        case "get_gpu_archive_names":
          return GPU_FIXTURES.map((gpu) => gpu.name);
        case "get_data_archive_records": {
          const a = args as {
            hardwareType: string;
            start: string;
            end: string;
          };
          return a.hardwareType === "cpu"
            ? buildArchiveRecords(a.start, a.end, 45, 20)
            : buildArchiveRecords(a.start, a.end, 60, 8);
        }
        case "get_gpu_archive_records": {
          const a = args as { dataType: string; start: string; end: string };
          if (a.dataType === "temp") {
            return buildArchiveRecords(a.start, a.end, 58, 6);
          }
          if (a.dataType === "dedicatedMemory") {
            return buildArchiveRecords(a.start, a.end, 4_194_304, 524_288);
          }
          return buildArchiveRecords(a.start, a.end, 55, 25);
        }
        case "get_process_stats": {
          const a = args as { endAt: string };
          return buildProcessStats(a.endAt);
        }
        case "get_process_stats_in_period": {
          const a = args as { end: string };
          return buildProcessStats(a.end);
        }
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
