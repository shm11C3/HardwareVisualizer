import type {
  HardwareMonitorUpdate,
  ProcessInfo_Serialize,
  StorageHealthRecord,
  StorageInfo,
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
  { pid: 100, name: "hv-fixture-app", cpuUsage: "12.5", memoryUsage: "256" },
  { pid: 200, name: "fixture-browser", cpuUsage: "8.1", memoryUsage: "1228.8" },
  { pid: 300, name: "fixture-editor", cpuUsage: "4.4", memoryUsage: "512" },
  { pid: 400, name: "fixture-daemon", cpuUsage: "1.2", memoryUsage: "64" },
  { pid: 500, name: "fixture-shell", cpuUsage: "0.3", memoryUsage: "32" },
];

export const buildStorageInfoFixture = (count: number): StorageInfo[] =>
  Array.from({ length: Math.max(0, count) }, (_, index) => {
    if (index === 0) {
      return {
        name: "Fixture SSD",
        size: 953,
        sizeUnit: "GB",
        free: 512,
        freeUnit: "GB",
        storageType: "ssd",
        fileSystem: "NTFS",
      };
    }

    if (index === 1) {
      return {
        name: "Fixture HDD",
        size: 3,
        sizeUnit: "GB",
        free: 1,
        freeUnit: "GB",
        storageType: "hdd",
        fileSystem: "NTFS",
      };
    }

    const size = index % 4 === 0 ? 4 : 1;
    const storageType = index % 5 === 0 ? "hdd" : "ssd";

    return {
      name: `Fixture ${storageType.toUpperCase()} ${index + 1}`,
      size,
      sizeUnit: "GB",
      free: Math.max(1, size - 1),
      freeUnit: "GB",
      storageType,
      fileSystem: index % 3 === 0 ? "exFAT" : "NTFS",
    };
  });

export const buildStorageHealthFixture = (
  count: number,
  options: {
    collectedAt?: string;
    date?: string;
  } = {},
): StorageHealthRecord[] =>
  Array.from({ length: Math.max(0, count) }, (_, index) => {
    const date = options.date ?? "2026-01-01";
    const isCritical = index > 0 && index % 7 === 0;
    const isWarning = !isCritical && index > 0 && index % 3 === 0;
    const healthStatus = isCritical
      ? "critical"
      : isWarning
        ? "warning"
        : "good";
    const displayName =
      index === 0
        ? "Fixture SSD"
        : index === 1
          ? "Fixture HDD"
          : `Fixture ${index % 5 === 0 ? "HDD" : "SSD"} ${index + 1}`;
    const temperatureCelsius = 36 + ((index * 3) % 29);
    const percentageUsed = Math.min(99, 3 + index * 4);
    const warningReasons = isCritical
      ? [
          `Fixture disk ${index + 1} has critical media errors`,
          `Temperature reached ${temperatureCelsius} C`,
        ]
      : isWarning
        ? [`Fixture disk ${index + 1} is above the warning threshold`]
        : [];

    return {
      deviceId: `e2e-disk-${index}`,
      displayName,
      model:
        index === 0
          ? "FIXTURE-SSD-1TB"
          : `FIXTURE-${index % 5 === 0 ? "HDD" : "SSD"}-${String(index + 1).padStart(2, "0")}`,
      protocol: index % 5 === 0 ? "SATA" : "NVMe",
      capacityBytes: (index % 4 === 0 ? 4 : 1) * 1_024_000_000_000,
      date,
      healthStatus,
      warningLevel: healthStatus === "good" ? "none" : healthStatus,
      temperatureCelsius,
      powerOnHours: 1234 + index * 911,
      percentageUsed,
      availableSparePercent: Math.max(1, 100 - index * 3),
      reallocatedSectorCount: isCritical ? 12 + index : null,
      currentPendingSectorCount: isWarning || isCritical ? index : null,
      offlineUncorrectableCount: isCritical ? 2 : null,
      mediaErrors: isCritical ? 5 + index : 0,
      errorLogEntries: isWarning || isCritical ? index * 2 : 0,
      unsafeShutdownCount: 2 + index,
      warningReasons,
      collectedAt: options.collectedAt ?? `${date}T00:00:00Z`,
    };
  });

export const storageHealthFixture: StorageHealthRecord[] =
  buildStorageHealthFixture(1);

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
    motherboardTemperatures: [
      {
        name: "SYSTIN",
        value: Math.round(38 + 4 * Math.sin(i / 6)),
        source: "NCT6799D / Super I/O",
      },
      {
        name: "CPUTIN",
        value: Math.round(48 + 8 * Math.sin(i / 5)),
        source: "NCT6799D / Super I/O",
      },
      {
        name: "AUXTIN0",
        value: Math.round(35 + 3 * Math.sin(i / 7)),
        source: "NCT6799D / Super I/O",
      },
    ],
    motherboardFanSpeeds: [
      {
        name: "Fan 1",
        rpm: Math.round(1200 + 120 * Math.sin(i / 4)),
        status: "active",
        source: "NCT6799D / Super I/O",
      },
      {
        name: "Fan 2",
        rpm: 0,
        status: "inactive",
        source: "NCT6799D / Super I/O",
      },
    ],
  }));

const round1 = (value: number) => Math.round(value * 10) / 10;
