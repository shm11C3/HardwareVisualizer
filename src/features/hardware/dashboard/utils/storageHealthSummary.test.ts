import { describe, expect, it } from "vitest";
import type { StorageHealthRecord } from "@/rspc/bindings";
import {
  buildStorageHealthSummary,
  formatStorageHealthMetricValue,
} from "./storageHealthSummary";

const record = (
  overrides: Partial<StorageHealthRecord> = {},
): StorageHealthRecord => ({
  deviceId: "storage:disk-a",
  displayName: "Disk A",
  model: "Example SSD",
  protocol: "NVMe",
  capacityBytes: 1_000_000,
  date: "2026-05-10",
  healthStatus: "good",
  warningLevel: "none",
  temperatureCelsius: 42,
  powerOnHours: 100,
  percentageUsed: null,
  availableSparePercent: null,
  reallocatedSectorCount: null,
  currentPendingSectorCount: null,
  offlineUncorrectableCount: null,
  mediaErrors: null,
  errorLogEntries: null,
  unsafeShutdownCount: null,
  warningReasons: [],
  collectedAt: "2026-05-10T09:12:00Z",
  ...overrides,
});

describe("buildStorageHealthSummary", () => {
  it("returns unknown when no records exist", () => {
    const summary = buildStorageHealthSummary(
      [],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.status).toBe("unknown");
    expect(summary.driveCount).toBe(0);
    expect(summary.focusDevice).toBeNull();
    expect(summary.devices).toEqual([]);
  });

  it("summarizes good records with drive count and max temperature", () => {
    const summary = buildStorageHealthSummary(
      [
        record({
          deviceId: "disk-a",
          displayName: "Disk A",
          temperatureCelsius: 38,
        }),
        record({
          deviceId: "disk-b",
          displayName: "Disk B",
          temperatureCelsius: 47,
        }),
        record({
          deviceId: "disk-c",
          displayName: "Disk C",
          temperatureCelsius: null,
        }),
      ],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.status).toBe("good");
    expect(summary.driveCount).toBe(3);
    expect(summary.maxTemperatureCelsius).toBe(47);
    expect(summary.lastCollectedAt).toBe("2026-05-10T09:12:00Z");
    expect(summary.devices).toHaveLength(3);
    expect(summary.devices[0]).toEqual({
      deviceId: "disk-a",
      label: "Example SSD",
      status: "good",
    });
  });

  it("selects warning devices and exposes reasons plus representative metrics", () => {
    const summary = buildStorageHealthSummary(
      [
        record({ deviceId: "disk-a", displayName: "Disk A" }),
        record({
          deviceId: "disk-b",
          displayName: "Samsung SSD 980 PRO",
          healthStatus: "warning",
          warningLevel: "warning",
          percentageUsed: 82,
          temperatureCelsius: 47,
          warningReasons: [
            "NVMe percentage used is high (82%)",
            "Temperature is elevated (47°C)",
            "extra reason",
          ],
        }),
      ],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.status).toBe("warning");
    expect(summary.focusDevice?.displayName).toBe("Samsung SSD 980 PRO");
    expect(summary.reasons).toEqual([
      "NVMe percentage used is high (82%)",
      "Temperature is elevated (47°C)",
    ]);
    expect(summary.metrics).toEqual([
      { type: "temperatureCelsius", value: 47 },
      { type: "percentageUsed", value: 82 },
      { type: "powerOnHours", value: 100 },
    ]);
    expect(summary.devices[0]).toEqual({
      deviceId: "disk-b",
      label: "Example SSD",
      status: "warning",
    });
  });

  it("exposes healthy storage health metrics when values were collected", () => {
    const summary = buildStorageHealthSummary(
      [
        record({
          temperatureCelsius: 44,
          percentageUsed: 0,
          availableSparePercent: 100,
          powerOnHours: 118,
        }),
      ],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.metrics).toEqual([
      { type: "temperatureCelsius", value: 44 },
      { type: "percentageUsed", value: 0 },
      { type: "availableSparePercent", value: 100 },
      { type: "powerOnHours", value: 118 },
    ]);
  });

  it("only exposes nonzero storage health counter metrics", () => {
    const summary = buildStorageHealthSummary(
      [
        record({
          currentPendingSectorCount: 0,
          offlineUncorrectableCount: 0,
          reallocatedSectorCount: 2,
          mediaErrors: 0,
          errorLogEntries: 3,
          unsafeShutdownCount: 6,
        }),
      ],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.metrics).toContainEqual({
      type: "reallocatedSectorCount",
      value: 2,
    });
    expect(summary.metrics).toContainEqual({
      type: "errorLogEntries",
      value: 3,
    });
    expect(summary.metrics).toContainEqual({
      type: "unsafeShutdownCount",
      value: 6,
    });
    expect(summary.metrics).not.toContainEqual({
      type: "currentPendingSectorCount",
      value: 0,
    });
    expect(summary.metrics).not.toContainEqual({
      type: "mediaErrors",
      value: 0,
    });
  });

  it("uses display name when the product model is unavailable", () => {
    const summary = buildStorageHealthSummary(
      [
        record({
          deviceId: "disk-a",
          displayName: "WDC WD40EZAZ",
          model: null,
        }),
      ],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.devices[0]?.label).toBe("WDC WD40EZAZ");
  });

  it("critical outranks warning", () => {
    const summary = buildStorageHealthSummary(
      [
        record({
          deviceId: "disk-a",
          displayName: "Warning Disk",
          healthStatus: "warning",
          warningLevel: "warning",
        }),
        record({
          deviceId: "disk-b",
          displayName: "Critical Disk",
          healthStatus: "critical",
          warningLevel: "critical",
          warningReasons: ["Current pending sectors are present"],
        }),
      ],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.status).toBe("critical");
    expect(summary.focusDevice?.displayName).toBe("Critical Disk");
  });

  it("keeps indeterminate records as unknown without treating them as missing", () => {
    const summary = buildStorageHealthSummary(
      [
        record({
          deviceId: "disk-a",
          displayName: "Unknown Disk",
          healthStatus: "unknown",
          warningLevel: "unknown",
        }),
      ],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.status).toBe("unknown");
    expect(summary.driveCount).toBe(1);
    expect(summary.isStale).toBe(false);
    expect(summary.focusDevice?.displayName).toBe("Unknown Disk");
    expect(summary.devices).toEqual([
      {
        deviceId: "disk-a",
        label: "Example SSD",
        status: "unknown",
      },
    ]);
  });

  it("marks old records as unknown", () => {
    const summary = buildStorageHealthSummary(
      [record({ date: "2026-05-07" })],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.status).toBe("unknown");
    expect(summary.isStale).toBe(true);
    expect(summary.focusDevice).toBeNull();
    expect(summary.devices).toEqual([
      {
        deviceId: "storage:disk-a",
        label: "Example SSD",
        status: "unknown",
      },
    ]);
  });

  it("excludes records from devices missing in the latest collection date", () => {
    const summary = buildStorageHealthSummary(
      [
        record({
          deviceId: "disk-a",
          displayName: "Current Disk",
          date: "2026-05-10",
          healthStatus: "good",
          warningLevel: "none",
        }),
        record({
          deviceId: "disk-b",
          displayName: "Removed Disk",
          date: "2026-05-08",
          healthStatus: "critical",
          warningLevel: "critical",
          warningReasons: ["Current pending sectors are present"],
        }),
      ],
      new Date("2026-05-10T10:00:00"),
    );

    expect(summary.status).toBe("good");
    expect(summary.driveCount).toBe(1);
    expect(summary.focusDevice?.displayName).toBe("Current Disk");
    expect(summary.devices).toEqual([
      {
        deviceId: "disk-a",
        label: "Example SSD",
        status: "good",
      },
    ]);
  });
});

describe("formatStorageHealthMetricValue", () => {
  it("formats dashboard storage health metric values with compact units", () => {
    expect(
      formatStorageHealthMetricValue({
        type: "temperatureCelsius",
        value: 44.4,
      }),
    ).toBe("44°C");
    expect(
      formatStorageHealthMetricValue({ type: "percentageUsed", value: 0 }),
    ).toBe("0%");
    expect(
      formatStorageHealthMetricValue({
        type: "availableSparePercent",
        value: 99.6,
      }),
    ).toBe("100%");
    expect(
      formatStorageHealthMetricValue({ type: "powerOnHours", value: 118 }),
    ).toBe("118 h");
    expect(
      formatStorageHealthMetricValue({ type: "mediaErrors", value: 3 }),
    ).toBe("3");
  });
});
