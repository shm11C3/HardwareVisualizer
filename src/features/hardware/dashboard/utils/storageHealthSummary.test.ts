import { describe, expect, it } from "vitest";
import type { StorageHealthRecord } from "@/rspc/bindings";
import { buildStorageHealthSummary } from "./storageHealthSummary";

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
      { type: "percentageUsed", value: 82 },
      { type: "temperatureCelsius", value: 47 },
    ]);
    expect(summary.devices[0]).toEqual({
      deviceId: "disk-b",
      label: "Example SSD",
      status: "warning",
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
});
