import type {
  LiveStorageHealth,
  StorageHealthRecord,
  StorageHealthStatus,
} from "@/rspc/bindings";

export const STORAGE_HEALTH_STALE_AFTER_DAYS = 2;

export type StorageHealthTemperatureSource = "live" | "record";

export type StorageHealthMetric =
  | {
      type: "temperatureCelsius";
      value: number;
      source: StorageHealthTemperatureSource;
      collectedAt: string;
    }
  | { type: "percentageUsed"; value: number }
  | { type: "availableSparePercent"; value: number }
  | { type: "powerOnHours"; value: number }
  | { type: "reallocatedSectorCount"; value: number }
  | { type: "currentPendingSectorCount"; value: number }
  | { type: "offlineUncorrectableCount"; value: number }
  | { type: "mediaErrors"; value: number }
  | { type: "errorLogEntries"; value: number }
  | { type: "unsafeShutdownCount"; value: number };

export type StorageHealthDeviceViewModel = {
  deviceId: string;
  label: string;
  protocolLabel: string | null;
  status: StorageHealthStatus;
};

export type StorageHealthSummaryViewModel = {
  status: StorageHealthStatus;
  driveCount: number;
  maxTemperatureCelsius: number | null;
  lastCollectedAt: string | null;
  latestDate: string | null;
  isStale: boolean;
  focusDevice: StorageHealthRecord | null;
  reasons: string[];
  metrics: StorageHealthMetric[];
  devices: StorageHealthDeviceViewModel[];
};

export type BuildStorageHealthSummaryOptions = {
  liveSignals?: LiveStorageHealth[];
  selectedDeviceId?: string | null;
};

const healthRank: Record<StorageHealthStatus, number> = {
  good: 0,
  unknown: 1,
  warning: 2,
  critical: 3,
};

export const buildStorageHealthSummary = (
  records: StorageHealthRecord[],
  now: Date = new Date(),
  options: BuildStorageHealthSummaryOptions = {},
): StorageHealthSummaryViewModel => {
  if (records.length === 0) {
    return {
      status: "unknown",
      driveCount: 0,
      maxTemperatureCelsius: null,
      lastCollectedAt: null,
      latestDate: null,
      isStale: false,
      focusDevice: null,
      reasons: [],
      metrics: [],
      devices: [],
    };
  }

  const latestDate = records.reduce(
    (latest, record) => (record.date > latest ? record.date : latest),
    records[0].date,
  );
  const recordsForDisplay = records.filter(
    (record) => record.date === latestDate,
  );
  const lastCollectedAt = recordsForDisplay.reduce(
    (latest, record) =>
      record.collectedAt > latest ? record.collectedAt : latest,
    recordsForDisplay[0].collectedAt,
  );
  const maxTemperatureCelsius = maxNumber(
    recordsForDisplay.map((record) => record.temperatureCelsius),
  );
  const isStale =
    daysBetweenDateKeys(toLocalDateKey(now), latestDate) >
    STORAGE_HEALTH_STALE_AFTER_DAYS;

  if (isStale) {
    return {
      status: "unknown",
      driveCount: recordsForDisplay.length,
      maxTemperatureCelsius,
      lastCollectedAt,
      latestDate,
      isStale: true,
      focusDevice: null,
      reasons: [],
      metrics: [],
      devices: buildDeviceList(recordsForDisplay, true),
    };
  }

  const orderedRecords = [...recordsForDisplay].sort((a, b) => {
    const rankDiff = healthRank[b.healthStatus] - healthRank[a.healthStatus];
    if (rankDiff !== 0) return rankDiff;
    return a.displayName.localeCompare(b.displayName);
  });
  const selectedDevice = options.selectedDeviceId
    ? orderedRecords.find(
        (record) => record.deviceId === options.selectedDeviceId,
      )
    : null;
  const focusDevice = selectedDevice ?? orderedRecords[0] ?? null;
  const liveSignalByDeviceId = new Map(
    (options.liveSignals ?? []).map((signal) => [signal.deviceId, signal]),
  );

  return {
    status: focusDevice?.healthStatus ?? "unknown",
    driveCount: recordsForDisplay.length,
    maxTemperatureCelsius,
    lastCollectedAt,
    latestDate,
    isStale: false,
    focusDevice,
    reasons: focusDevice?.warningReasons.slice(0, 2) ?? [],
    metrics: focusDevice
      ? collectMetrics(
          focusDevice,
          liveSignalByDeviceId.get(focusDevice.deviceId),
        )
      : [],
    devices: buildDeviceList(orderedRecords, false),
  };
};

const buildDeviceList = (
  records: StorageHealthRecord[],
  forceUnknown: boolean,
): StorageHealthDeviceViewModel[] => {
  return records.map((record) => ({
    deviceId: record.deviceId,
    label: record.model?.trim() || record.displayName,
    protocolLabel: formatStorageDeviceProtocolLabel(record.protocol),
    status: forceUnknown ? "unknown" : record.healthStatus,
  }));
};

export const formatStorageDeviceProtocolLabel = (
  protocol: string | null | undefined,
): string | null => {
  const normalized = protocol?.trim();
  if (!normalized) return null;

  const lower = normalized.toLowerCase();
  if (lower.includes("nvme")) return "NVMe";
  if (lower.includes("sata") || lower === "sat" || lower.startsWith("sat,")) {
    return "SATA";
  }
  if (lower === "ata" || lower.startsWith("ata,")) return "ATA";
  if (lower.includes("scsi")) return "SCSI";
  if (lower.includes("usb")) return "USB";
  if (lower.includes("raid")) return "RAID";
  if (lower.includes("mmc")) return "eMMC";

  return null;
};

const collectMetrics = (
  record: StorageHealthRecord,
  liveSignal?: LiveStorageHealth,
): StorageHealthMetric[] => {
  const metrics: StorageHealthMetric[] = [];

  const temperature =
    liveSignal?.temperatureCelsius ?? record.temperatureCelsius;
  if (temperature != null) {
    metrics.push({
      type: "temperatureCelsius",
      value: temperature,
      source: liveSignal?.temperatureCelsius != null ? "live" : "record",
      collectedAt:
        liveSignal?.temperatureCelsius != null
          ? liveSignal.collectedAt
          : record.collectedAt,
    });
  }
  if (record.percentageUsed != null) {
    metrics.push({ type: "percentageUsed", value: record.percentageUsed });
  }
  if (record.availableSparePercent != null) {
    metrics.push({
      type: "availableSparePercent",
      value: record.availableSparePercent,
    });
  }
  if (record.powerOnHours != null) {
    metrics.push({
      type: "powerOnHours",
      value: record.powerOnHours,
    });
  }
  if (hasCount(record.currentPendingSectorCount)) {
    metrics.push({
      type: "currentPendingSectorCount",
      value: record.currentPendingSectorCount,
    });
  }
  if (hasCount(record.offlineUncorrectableCount)) {
    metrics.push({
      type: "offlineUncorrectableCount",
      value: record.offlineUncorrectableCount,
    });
  }
  if (hasCount(record.reallocatedSectorCount)) {
    metrics.push({
      type: "reallocatedSectorCount",
      value: record.reallocatedSectorCount,
    });
  }
  if (hasCount(record.mediaErrors)) {
    metrics.push({ type: "mediaErrors", value: record.mediaErrors });
  }
  if (hasCount(record.errorLogEntries)) {
    metrics.push({ type: "errorLogEntries", value: record.errorLogEntries });
  }
  if (hasCount(record.unsafeShutdownCount)) {
    metrics.push({
      type: "unsafeShutdownCount",
      value: record.unsafeShutdownCount,
    });
  }

  return metrics;
};

const hasCount = (value: number | null | undefined): value is number =>
  value != null && value > 0;

export const formatStorageHealthMetricValue = (
  metric: StorageHealthMetric,
): string => {
  switch (metric.type) {
    case "temperatureCelsius":
      return `${Math.round(metric.value)}°C`;
    case "percentageUsed":
    case "availableSparePercent":
      return `${Math.round(metric.value)}%`;
    case "powerOnHours":
      return `${Math.round(metric.value)} h`;
    case "reallocatedSectorCount":
    case "currentPendingSectorCount":
    case "offlineUncorrectableCount":
    case "mediaErrors":
    case "errorLogEntries":
    case "unsafeShutdownCount":
      return `${Math.round(metric.value)}`;
  }
};

const maxNumber = (values: Array<number | null | undefined>) => {
  const numbers = values.filter((value): value is number => value != null);
  return numbers.length > 0 ? Math.max(...numbers) : null;
};

const toLocalDateKey = (date: Date) => {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
};

const parseDateKeyUtc = (dateKey: string) => {
  const [year, month, day] = dateKey.split("-").map(Number);
  if (!year || !month || !day) return Number.NaN;
  return Date.UTC(year, month - 1, day);
};

const daysBetweenDateKeys = (currentDate: string, recordDate: string) => {
  const current = parseDateKeyUtc(currentDate);
  const record = parseDateKeyUtc(recordDate);
  if (!Number.isFinite(current) || !Number.isFinite(record)) {
    return Number.POSITIVE_INFINITY;
  }
  return Math.floor((current - record) / 86_400_000);
};
