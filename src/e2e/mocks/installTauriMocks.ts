import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type {
  ArchiveBucketTimestamp,
  HardwareMonitorUpdate,
} from "@/rspc/bindings";
import { buildArchiveSeries, buildProcessStats } from "../fixtures/archive";
import {
  buildCoolingDailyTrendFixture,
  coolingBandComparisonEstablishingFixture,
  coolingBandComparisonFixture,
  coolingBaselineDeltaEstablishingFixture,
  coolingBaselineDeltaFixture,
  coolingBaselineDeltaLargeRiseFixture,
  coolingBaselineDeltaMildRiseFixture,
  coolingBaselineDeltaNotComparableFixture,
  coolingLoadTemperatureExplorerEstablishingFixture,
  coolingLoadTemperatureExplorerFixture,
} from "../fixtures/cooling";
import {
  buildHardwareUpdateSeries,
  buildStorageHealthFixture,
  buildStorageInfoFixture,
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
      /** Number of mocked IPC invocations observed for a command. */
      getInvokeCount: (command: string) => number;
      /**
       * Start a deterministic event stream through the mocked Tauri event IPC.
       * Used by long-running frontend memory tests.
       */
      startHardwareUpdateStream: (options?: {
        intervalMs?: number;
      }) => Promise<void>;
      stopHardwareUpdateStream: () => Promise<{ emittedCount: number }>;
    };
  }
}

type InvokeHandler = (args?: unknown) => unknown;
type EventListenArgs = { event: string; handler: number };
type EventEmitArgs = { event: string; payload?: unknown };
type EventUnlistenArgs = { event: string; eventId?: number; id?: number };
type FixtureOverrides = {
  storageDeviceCount: number | null;
  showNavigationNotice: boolean;
  classicNavigation: boolean;
  /** Seeds `store.json`'s `display` so upgrade paths can be exercised. */
  storedDisplayTarget: string | null;
  /** `?coolingBaseline=establishing` switches the Cooling tab's idle-baseline
   * fixtures (`get_cooling_baseline_delta`/`get_cooling_band_comparison`)
   * from "established" to "establishing" so both empty states are
   * reachable from a URL. */
  coolingBaselineEstablishing: boolean;
  /** `?coolingObservation=notComparable|sustainedMildRise|sustainedLargeRise`
   * swaps `get_cooling_baseline_delta`'s established-baseline fixture to
   * exercise the other `CoolingDeltaObservation` states from a URL. Ignored
   * when `coolingBaselineEstablishing` is set; unset/unrecognized falls
   * back to the default `withinRange` fixture. */
  coolingObservationOverride: CoolingObservationOverride;
  /** `?coolingPower=none` empties every CPU-power source the Cooling tab
   * reads (the archive `cpuPower` series and the daily trend's `power`),
   * simulating a machine whose platform publishes no CPU package power.
   * The timeline then draws no power lane and the pending-sensors note
   * keeps naming power (#2021). */
  coolingPowerUnsupported: boolean;
};
type CoolingObservationOverride =
  | "notComparable"
  | "sustainedMildRise"
  | "sustainedLargeRise"
  | null;
type TauriInternalsWindow = Window & {
  __TAURI_INTERNALS__?: {
    runCallback?: (id: number, data: unknown) => void;
  };
};

const STORE_RID = 1;
const MAX_STORAGE_DEVICE_STUB_COUNT = 32;

const storeKey = (args?: unknown) => (args as { key: string }).key;

const readFixtureOverrides = (): FixtureOverrides => {
  const rawStorageDeviceCount = new URLSearchParams(window.location.search).get(
    "storageDevices",
  );
  const parsedStorageDeviceCount =
    rawStorageDeviceCount == null
      ? Number.NaN
      : Number.parseInt(rawStorageDeviceCount, 10);

  return {
    storageDeviceCount: Number.isFinite(parsedStorageDeviceCount)
      ? Math.max(
          0,
          Math.min(MAX_STORAGE_DEVICE_STUB_COUNT, parsedStorageDeviceCount),
        )
      : null,
    showNavigationNotice:
      new URLSearchParams(window.location.search).get(
        "showNavigationNotice",
      ) === "1",
    classicNavigation:
      new URLSearchParams(window.location.search).get("navigationLayout") ===
      "classic",
    storedDisplayTarget: new URLSearchParams(window.location.search).get(
      "storedDisplay",
    ),
    coolingBaselineEstablishing:
      new URLSearchParams(window.location.search).get("coolingBaseline") ===
      "establishing",
    coolingObservationOverride: readCoolingObservationOverride(),
    coolingPowerUnsupported:
      new URLSearchParams(window.location.search).get("coolingPower") ===
      "none",
  };
};

const readCoolingObservationOverride = (): CoolingObservationOverride => {
  const raw = new URLSearchParams(window.location.search).get(
    "coolingObservation",
  );
  return raw === "notComparable" ||
    raw === "sustainedMildRise" ||
    raw === "sustainedLargeRise"
    ? raw
    : null;
};

/**
 * Dispatch table mapping invoke commands to their mocked handlers:
 * Tauri plugin commands (`plugin:<name>|<command>`) and generated
 * tauri-specta commands (raw or typedError-boxed by the bindings).
 */
const buildInvokeHandlers = (
  store: Map<string, unknown>,
  eventListeners: Map<string, Set<number>>,
  fixtureOverrides: FixtureOverrides,
): Record<string, InvokeHandler> => ({
  // --- @tauri-apps/plugin-event ---
  "plugin:event|listen": (args) => {
    const a = args as EventListenArgs;
    if (!eventListeners.has(a.event)) {
      eventListeners.set(a.event, new Set());
    }
    eventListeners.get(a.event)?.add(a.handler);
    return a.handler;
  },
  "plugin:event|emit": (args) => {
    dispatchTauriEvent(eventListeners, args as EventEmitArgs);
    return null;
  },
  "plugin:event|unlisten": (args) => {
    const a = args as EventUnlistenArgs;
    eventListeners.get(a.event)?.delete(a.eventId ?? a.id ?? -1);
    return null;
  },

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
  // clear() empties the store; reset() restores the configured defaults
  // (falling back to clear() when no defaults exist) — see plugin-store v2.
  "plugin:store|clear": () => {
    store.clear();
    return null;
  },
  "plugin:store|reset": () => {
    store.clear();
    for (const [key, value] of Object.entries(storeFixture)) {
      store.set(key, value);
    }
    return null;
  },

  // --- window/plugin surface ---
  "plugin:window|theme": () => "dark",
  "plugin:dialog|message": () => null,
  "plugin:autostart|is_enabled": () => false,
  "plugin:app|version": () => "1.0.0",

  // --- generated commands ---
  get_settings: () => ({
    ...settingsFixture,
    navigationLayout: fixtureOverrides.classicNavigation
      ? "classic"
      : settingsFixture.navigationLayout,
    uiAnnouncementVersion: fixtureOverrides.showNavigationNotice
      ? 0
      : settingsFixture.uiAnnouncementVersion,
  }),
  get_hardware_info: () =>
    fixtureOverrides.storageDeviceCount == null
      ? sysInfoFixture
      : {
          ...sysInfoFixture,
          storage: buildStorageInfoFixture(fixtureOverrides.storageDeviceCount),
        },
  get_process_list: () => processListFixture,
  get_storage_health_latest_records: () =>
    fixtureOverrides.storageDeviceCount == null
      ? storageHealthFixture
      : buildStorageHealthFixture(fixtureOverrides.storageDeviceCount, {
          date: new Date().toISOString().slice(0, 10),
        }),
  get_live_storage_health: () => [],
  refresh_storage_devices: () =>
    fixtureOverrides.storageDeviceCount == null
      ? storageHealthFixture
      : buildStorageHealthFixture(fixtureOverrides.storageDeviceCount, {
          date: new Date().toISOString().slice(0, 10),
        }),
  get_external_component_guidance_candidates: () => [],
  defer_external_component_guidance_for_session: () => null,
  acknowledge_external_component_guidance_key: () => null,
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
  get_data_archive_series: (args) => {
    const a = args as {
      hardwareType: string;
      dataStats: string;
      start: string;
      end: string;
      bucketWidthMs: number;
      bucketTimestamp: ArchiveBucketTimestamp;
    };
    // Separate the avg/max/min series the way a real archive does, so a
    // chart drawing all three (the Cooling tab's temperature lane) shows a
    // real band instead of three identical curves.
    const spread = (width: number) =>
      a.dataStats === "max" ? width : a.dataStats === "min" ? -width : 0;

    if (a.hardwareType === "cpu") {
      return buildArchiveSeries(
        a.start,
        a.end,
        a.bucketWidthMs,
        a.bucketTimestamp,
        45 + spread(18),
        20,
        // The CPU series shares the temperature series' gaps so the Cooling
        // tab's two lanes break at the same buckets.
        { gapEvery: 17 },
      );
    }
    if (a.hardwareType === "cpuTemperature") {
      return buildArchiveSeries(
        a.start,
        a.end,
        a.bucketWidthMs,
        a.bucketTimestamp,
        58 + spread(7),
        6,
        { gapEvery: 17 },
      );
    }
    if (a.hardwareType.endsWith("Power")) {
      if (fixtureOverrides.coolingPowerUnsupported) {
        // The archive returns no buckets at all when nothing was ever
        // recorded - not buckets holding zero.
        return [];
      }
      return buildArchiveSeries(
        a.start,
        a.end,
        a.bucketWidthMs,
        a.bucketTimestamp,
        18 + spread(5),
        7,
        // The power series is archived from the same one-minute rows as
        // the temperature and CPU series, so it must break at the same
        // buckets - the Cooling tab's three lanes share one axis.
        { gapEvery: 17 },
      );
    }
    return buildArchiveSeries(
      a.start,
      a.end,
      a.bucketWidthMs,
      a.bucketTimestamp,
      60 + spread(10),
      8,
    );
  },
  get_gpu_archive_series: (args) => {
    const a = args as {
      dataType: string;
      start: string;
      end: string;
      bucketWidthMs: number;
      bucketTimestamp: ArchiveBucketTimestamp;
    };
    if (a.dataType === "temp") {
      return buildArchiveSeries(
        a.start,
        a.end,
        a.bucketWidthMs,
        a.bucketTimestamp,
        58,
        6,
      );
    }
    if (a.dataType === "dedicatedMemory") {
      return buildArchiveSeries(
        a.start,
        a.end,
        a.bucketWidthMs,
        a.bucketTimestamp,
        4_194_304,
        524_288,
      );
    }
    return buildArchiveSeries(
      a.start,
      a.end,
      a.bucketWidthMs,
      a.bucketTimestamp,
      55,
      25,
    );
  },
  get_process_stats: (args) =>
    buildProcessStats((args as { endAt: string }).endAt),
  get_process_stats_in_period: (args) =>
    buildProcessStats((args as { end: string }).end),

  // --- cooling insight commands (#2018) ---
  get_cooling_trend: (args) =>
    buildCoolingDailyTrendFixture(
      (args as { days: number }).days,
      undefined,
      !fixtureOverrides.coolingPowerUnsupported,
    ),
  get_cooling_band_comparison: () =>
    fixtureOverrides.coolingBaselineEstablishing
      ? coolingBandComparisonEstablishingFixture
      : coolingBandComparisonFixture,
  // The Explorer (#2023) only invokes this once expanded, so a capture
  // that never opens it must never reach this handler.
  get_cooling_load_temperature_explorer: () =>
    fixtureOverrides.coolingBaselineEstablishing
      ? coolingLoadTemperatureExplorerEstablishingFixture
      : coolingLoadTemperatureExplorerFixture,
  get_cooling_baseline_delta: () => {
    if (fixtureOverrides.coolingBaselineEstablishing) {
      return coolingBaselineDeltaEstablishingFixture;
    }
    switch (fixtureOverrides.coolingObservationOverride) {
      case "notComparable":
        return coolingBaselineDeltaNotComparableFixture;
      case "sustainedMildRise":
        return coolingBaselineDeltaMildRiseFixture;
      case "sustainedLargeRise":
        return coolingBaselineDeltaLargeRiseFixture;
      default:
        return coolingBaselineDeltaFixture;
    }
  },
});

const dispatchTauriEvent = (
  eventListeners: Map<string, Set<number>>,
  args: EventEmitArgs,
) => {
  const runCallback = (window as TauriInternalsWindow).__TAURI_INTERNALS__
    ?.runCallback;
  if (!runCallback) {
    return;
  }

  for (const handler of eventListeners.get(args.event) ?? []) {
    runCallback(handler, {
      event: args.event,
      id: handler,
      payload: args.payload,
    });
  }
};

/**
 * Install Tauri IPC/event/window mocks so the React app runs in a plain
 * browser with deterministic fixture data. Loaded from `src/main.e2e.tsx`,
 * which Vite serves only in `--mode e2e`.
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
  const eventListeners = new Map<string, Set<number>>();
  const fixtureOverrides = readFixtureOverrides();
  if (fixtureOverrides.storedDisplayTarget != null) {
    store.set("display", fixtureOverrides.storedDisplayTarget);
  }
  const handlers = buildInvokeHandlers(store, eventListeners, fixtureOverrides);
  const invokeCounts = new Map<string, number>();
  let streamTimer: number | undefined;
  let streamIndex = 0;
  let streamRunning = false;

  const emitHardwareUpdateAt = async (index: number) => {
    const series = buildHardwareUpdateSeries(index + 1);
    dispatchTauriEvent(eventListeners, {
      event: "hardware-monitor-update",
      payload: series[index],
    });
  };

  const stopHardwareUpdateStream = () => {
    streamRunning = false;
    if (streamTimer !== undefined) {
      window.clearTimeout(streamTimer);
      streamTimer = undefined;
    }
    return { emittedCount: streamIndex };
  };

  mockIPC((cmd: string, args?: unknown) => {
    invokeCounts.set(cmd, (invokeCounts.get(cmd) ?? 0) + 1);

    if (Object.hasOwn(handlers, cmd)) {
      return handlers[cmd](args);
    }

    // Settings mutators (`set_theme`, `set_language`, ...) succeed silently
    // so settings-screen scenarios can interact without enumerating them.
    if (cmd.startsWith("set_")) {
      return null;
    }

    if (cmd === "acknowledge_navigation_restructure_announcement") {
      return null;
    }

    throw new Error(`[e2e-mock] Unhandled invoke: ${cmd}`);
  });

  window.__E2E__ = {
    getInvokeCount: (command) => invokeCounts.get(command) ?? 0,
    emitHardwareUpdate: async (payload) =>
      dispatchTauriEvent(eventListeners, {
        event: "hardware-monitor-update",
        payload: payload ?? buildHardwareUpdateSeries(1)[0],
      }),
    emitHardwareUpdateSeries: async (count = 30) => {
      for (const payload of buildHardwareUpdateSeries(count)) {
        dispatchTauriEvent(eventListeners, {
          event: "hardware-monitor-update",
          payload,
        });
      }
    },
    startHardwareUpdateStream: async ({ intervalMs = 1_000 } = {}) => {
      stopHardwareUpdateStream();
      streamIndex = 0;
      streamRunning = true;

      const tick = async () => {
        if (!streamRunning) return;

        await emitHardwareUpdateAt(streamIndex);
        streamIndex += 1;

        if (streamRunning) {
          streamTimer = window.setTimeout(tick, intervalMs);
        }
      };

      await tick();
    },
    stopHardwareUpdateStream: async () => stopHardwareUpdateStream(),
  };
};
