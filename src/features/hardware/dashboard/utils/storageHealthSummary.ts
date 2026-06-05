import type {
  StorageHealthStatus,
  StorageSmartDashboardSnapshot,
} from "@/rspc/bindings";

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
  focusDevice: StorageSmartDashboardSnapshot | null;
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
  snapshots: StorageSmartDashboardSnapshot[],
  now: Date = new Date(),
): StorageHealthSummaryViewModel => {
  if (snapshots.length === 0) {
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

  const latestDate = snapshots.reduce(
    (latest, snapshot) => (snapshot.date > latest ? snapshot.date : latest),
    snapshots[0].date,
  );
  const lastCollectedAt = snapshots.reduce(
    (latest, snapshot) =>
      snapshot.collectedAt > latest ? snapshot.collectedAt : latest,
    snapshots[0].collectedAt,
  );
  const maxTemperatureCelsius = maxNumber(
    snapshots.map((snapshot) => snapshot.temperatureCelsius),
  );
  const isStale =
    daysBetweenDateKeys(toLocalDateKey(now), latestDate) >
    STORAGE_HEALTH_STALE_AFTER_DAYS;

  if (isStale) {
    return {
      status: "unknown",
      driveCount: snapshots.length,
      maxTemperatureCelsius,
      lastCollectedAt,
      latestDate,
      isStale: true,
      focusDevice: null,
      reasons: [],
      metrics: [],
      devices: buildDeviceList(snapshots, true),
    };
  }

  const orderedSnapshots = [...snapshots].sort((a, b) => {
    const rankDiff = healthRank[b.healthStatus] - healthRank[a.healthStatus];
    if (rankDiff !== 0) return rankDiff;
    return a.displayName.localeCompare(b.displayName);
  });
  const focusDevice = orderedSnapshots[0] ?? null;

  return {
    status: focusDevice?.healthStatus ?? "unknown",
    driveCount: snapshots.length,
    maxTemperatureCelsius,
    lastCollectedAt,
    latestDate,
    isStale: false,
    focusDevice,
    reasons: focusDevice?.warningReasons.slice(0, 2) ?? [],
    metrics: focusDevice
      ? collectMetrics(focusDevice, maxTemperatureCelsius)
      : [],
    devices: buildDeviceList(orderedSnapshots, false),
  };
};

const buildDeviceList = (
  snapshots: StorageSmartDashboardSnapshot[],
  forceUnknown: boolean,
): StorageHealthDeviceViewModel[] => {
  return snapshots.map((snapshot) => ({
    deviceId: snapshot.deviceId,
    label: snapshot.model?.trim() || snapshot.displayName,
    status: forceUnknown ? "unknown" : snapshot.healthStatus,
  }));
};

const collectMetrics = (
  snapshot: StorageSmartDashboardSnapshot,
  maxTemperatureCelsius: number | null,
): StorageHealthMetric[] => {
  const metrics: StorageHealthMetric[] = [];

  if (snapshot.percentageUsed != null) {
    metrics.push({ type: "percentageUsed", value: snapshot.percentageUsed });
  }
  if (maxTemperatureCelsius != null) {
    metrics.push({
      type: "temperatureCelsius",
      value: maxTemperatureCelsius,
    });
  }
  if (snapshot.availableSparePercent != null) {
    metrics.push({
      type: "availableSparePercent",
      value: snapshot.availableSparePercent,
    });
  }
  if (snapshot.currentPendingSectorCount != null) {
    metrics.push({
      type: "currentPendingSectorCount",
      value: snapshot.currentPendingSectorCount,
    });
  }
  if (snapshot.offlineUncorrectableCount != null) {
    metrics.push({
      type: "offlineUncorrectableCount",
      value: snapshot.offlineUncorrectableCount,
    });
  }
  if (snapshot.reallocatedSectorCount != null) {
    metrics.push({
      type: "reallocatedSectorCount",
      value: snapshot.reallocatedSectorCount,
    });
  }
  if (snapshot.mediaErrors != null) {
    metrics.push({ type: "mediaErrors", value: snapshot.mediaErrors });
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

const daysBetweenDateKeys = (currentDate: string, snapshotDate: string) => {
  const current = parseDateKeyUtc(currentDate);
  const snapshot = parseDateKeyUtc(snapshotDate);
  if (!Number.isFinite(current) || !Number.isFinite(snapshot)) {
    return Number.POSITIVE_INFINITY;
  }
  return Math.floor((current - snapshot) / 86_400_000);
};
