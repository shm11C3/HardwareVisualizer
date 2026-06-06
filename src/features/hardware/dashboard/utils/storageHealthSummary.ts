import type { StorageHealthRecord, StorageHealthStatus } from "@/rspc/bindings";

export const STORAGE_HEALTH_STALE_AFTER_DAYS = 2;

export type StorageHealthMetric =
  | { type: "percentageUsed"; value: number }
  | { type: "temperatureCelsius"; value: number }
  | { type: "availableSparePercent"; value: number }
  | { type: "reallocatedSectorCount"; value: number }
  | { type: "currentPendingSectorCount"; value: number }
  | { type: "offlineUncorrectableCount"; value: number }
  | { type: "mediaErrors"; value: number };

export type StorageHealthDeviceViewModel = {
  deviceId: string;
  label: string;
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

const healthRank: Record<StorageHealthStatus, number> = {
  good: 0,
  unknown: 1,
  warning: 2,
  critical: 3,
};

export const buildStorageHealthSummary = (
  records: StorageHealthRecord[],
  now: Date = new Date(),
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
  const lastCollectedAt = records.reduce(
    (latest, record) =>
      record.collectedAt > latest ? record.collectedAt : latest,
    records[0].collectedAt,
  );
  const maxTemperatureCelsius = maxNumber(
    records.map((record) => record.temperatureCelsius),
  );
  const isStale =
    daysBetweenDateKeys(toLocalDateKey(now), latestDate) >
    STORAGE_HEALTH_STALE_AFTER_DAYS;

  if (isStale) {
    return {
      status: "unknown",
      driveCount: records.length,
      maxTemperatureCelsius,
      lastCollectedAt,
      latestDate,
      isStale: true,
      focusDevice: null,
      reasons: [],
      metrics: [],
      devices: buildDeviceList(records, true),
    };
  }

  const orderedRecords = [...records].sort((a, b) => {
    const rankDiff = healthRank[b.healthStatus] - healthRank[a.healthStatus];
    if (rankDiff !== 0) return rankDiff;
    return a.displayName.localeCompare(b.displayName);
  });
  const focusDevice = orderedRecords[0] ?? null;

  return {
    status: focusDevice?.healthStatus ?? "unknown",
    driveCount: records.length,
    maxTemperatureCelsius,
    lastCollectedAt,
    latestDate,
    isStale: false,
    focusDevice,
    reasons: focusDevice?.warningReasons.slice(0, 2) ?? [],
    metrics: focusDevice
      ? collectMetrics(focusDevice, maxTemperatureCelsius)
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
    status: forceUnknown ? "unknown" : record.healthStatus,
  }));
};

const collectMetrics = (
  record: StorageHealthRecord,
  maxTemperatureCelsius: number | null,
): StorageHealthMetric[] => {
  const metrics: StorageHealthMetric[] = [];

  if (record.percentageUsed != null) {
    metrics.push({ type: "percentageUsed", value: record.percentageUsed });
  }
  if (maxTemperatureCelsius != null) {
    metrics.push({
      type: "temperatureCelsius",
      value: maxTemperatureCelsius,
    });
  }
  if (record.availableSparePercent != null) {
    metrics.push({
      type: "availableSparePercent",
      value: record.availableSparePercent,
    });
  }
  if (record.currentPendingSectorCount != null) {
    metrics.push({
      type: "currentPendingSectorCount",
      value: record.currentPendingSectorCount,
    });
  }
  if (record.offlineUncorrectableCount != null) {
    metrics.push({
      type: "offlineUncorrectableCount",
      value: record.offlineUncorrectableCount,
    });
  }
  if (record.reallocatedSectorCount != null) {
    metrics.push({
      type: "reallocatedSectorCount",
      value: record.reallocatedSectorCount,
    });
  }
  if (record.mediaErrors != null) {
    metrics.push({ type: "mediaErrors", value: record.mediaErrors });
  }

  return metrics.slice(0, 2);
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
