import type {
  HardwareMonitorUpdate,
  ProcessInfo_Serialize,
  StorageHealthRecord,
  SysInfo,
} from "@/rspc/bindings";

/**
 * GPU ids must match between `sysInfoFixture.gpus[].id` and the
 * `hardware-monitor-update` payloads (`gpus[].gpuId`) — the dashboard joins
 * them to resolve the selected GPU. Two GPUs are provided so the GPU selector
 * tablist renders and can be driven via accessible selectors.
 */
export const GPU_FIXTURES = [
  { id: "e2e-gpu-0", name: "HV Fixture GPU 8GB" },
  { id: "e2e-gpu-1", name: "HV Fixture iGPU" },
] as const;

export const sysInfoFixture: SysInfo = {
  cpu: {
    name: "HV Fixture CPU 8-Core",
    vendor: "FixtureVendor",
    coreCount: 8,
    clock: 3600,
    clockUnit: "MHz",
    cpuName: "HV Fixture CPU 8-Core",
  },
  memory: {
    size: "32 GB",
    clock: 4800,
    clockUnit: "MHz",
    memoryCount: 2,
    totalSlots: 4,
    memoryType: "DDR5",
    isDetailed: false,
  },
  gpus: [
    {
      id: GPU_FIXTURES[0].id,
      name: GPU_FIXTURES[0].name,
      vendorName: "FixtureVendor",
      clock: 2100,
      memorySize: "8 GB",
      memorySizeDedicated: "8 GB",
      coreCount: null,
    },
    {
      id: GPU_FIXTURES[1].id,
      name: GPU_FIXTURES[1].name,
      vendorName: "FixtureVendor",
      clock: 1500,
      memorySize: "2 GB",
      memorySizeDedicated: "2 GB",
      coreCount: null,
    },
  ],
  storage: [
    {
      name: "Fixture SSD",
      size: 953,
      sizeUnit: "GB",
      free: 512,
      freeUnit: "GB",
      storageType: "ssd",
      fileSystem: "NTFS",
    },
    {
      name: "Fixture HDD",
      size: 3,
      sizeUnit: "GB",
      free: 1,
      freeUnit: "GB",
      storageType: "hdd",
      fileSystem: "NTFS",
    },
  ],
  motherboard: {
    manufacturer: "FixtureVendor",
    product: "HV Fixture Board",
    version: "1.0",
    serialNumber: "E2E-0000",
    biosVendor: "FixtureVendor",
    biosVersion: "1.0.0",
    biosReleaseDate: "2024-01-01",
  },
};

export const processListFixture: ProcessInfo_Serialize[] = [
  { pid: 100, name: "hv-fixture-app", cpuUsage: "12.5", memoryUsage: "256 MB" },
  { pid: 200, name: "fixture-browser", cpuUsage: "8.1", memoryUsage: "1.2 GB" },
  { pid: 300, name: "fixture-editor", cpuUsage: "4.4", memoryUsage: "512 MB" },
  { pid: 400, name: "fixture-daemon", cpuUsage: "1.2", memoryUsage: "64 MB" },
  { pid: 500, name: "fixture-shell", cpuUsage: "0.3", memoryUsage: "32 MB" },
];

export const storageHealthFixture: StorageHealthRecord[] = [
  {
    deviceId: "e2e-disk-0",
    displayName: "Fixture SSD",
    model: "FIXTURE-SSD-1TB",
    protocol: "NVMe",
    capacityBytes: 1_024_000_000_000,
    date: "2026-01-01",
    healthStatus: "good",
    warningLevel: "none",
    temperatureCelsius: 38,
    powerOnHours: 1234,
    percentageUsed: 3,
    availableSparePercent: 100,
    reallocatedSectorCount: null,
    currentPendingSectorCount: null,
    offlineUncorrectableCount: null,
    mediaErrors: 0,
    errorLogEntries: 0,
    unsafeShutdownCount: 2,
    warningReasons: [],
    collectedAt: "2026-01-01T00:00:00Z",
  },
];

/**
 * Build a deterministic series of `hardware-monitor-update` payloads.
 * Values follow fixed sine waves so charts always render the same shape.
 */
export const buildHardwareUpdateSeries = (
  length: number,
): HardwareMonitorUpdate[] =>
  Array.from({ length }, (_, i) => ({
    cpuUsage: round1(45 + 25 * Math.sin(i / 4)),
    memoryUsage: round1(62 + 6 * Math.sin(i / 6 + 1)),
    gpus: [
      {
        gpuId: GPU_FIXTURES[0].id,
        gpuName: GPU_FIXTURES[0].name,
        gpuUsage: round1(55 + 35 * Math.sin(i / 3 + 2)),
        gpuTemperature: round1(58 + 6 * Math.sin(i / 5)),
        gpuSource: "fixture",
        gpuDedicatedMemoryUsageKb: 4_194_304,
        gpuCoolerLevel: 42,
      },
      {
        gpuId: GPU_FIXTURES[1].id,
        gpuName: GPU_FIXTURES[1].name,
        gpuUsage: round1(20 + 10 * Math.sin(i / 4 + 1)),
        gpuTemperature: round1(45 + 4 * Math.sin(i / 5 + 1)),
        gpuSource: "fixture",
        gpuDedicatedMemoryUsageKb: 1_048_576,
        gpuCoolerLevel: 30,
      },
    ],
    processorsUsage: Array.from({ length: 8 }, (_, core) =>
      round1(40 + 30 * Math.sin(i / 4 + core)),
    ),
    cpuTemperature: Math.round(50 + 8 * Math.sin(i / 5)),
    sensorTemperatures: [
      { name: "CPUZ", value: Math.round(50 + 8 * Math.sin(i / 5)) },
      { name: "TZ01", value: Math.round(42 + 5 * Math.sin(i / 6)) },
    ],
  }));

const round1 = (value: number) => Math.round(value * 10) / 10;
