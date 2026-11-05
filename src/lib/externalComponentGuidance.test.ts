import { describe, expect, it } from "vitest";
import type { ExternalComponentGuidanceCandidate } from "@/rspc/bindings";
import {
  externalComponentGuidanceActionKey,
  externalComponentGuidanceCopyKey,
  externalComponentGuidanceDocsBaseUrl,
  externalComponentGuidanceDocsUrl,
} from "./externalComponentGuidance";

const candidate = (
  key: string,
  overrides: Partial<ExternalComponentGuidanceCandidate> = {},
): ExternalComponentGuidanceCandidate => ({
  key,
  component: "pawnio",
  usage: "cpuPackageTemperature",
  reasonKind: "missing",
  missingSignals: [],
  affectedDeviceCount: null,
  diagnosticDetail: null,
  ...overrides,
});

describe("externalComponentGuidance", () => {
  it("maps stable candidate keys to copy keys and docs URLs", () => {
    const pawnio = candidate("pawnio:cpu-package-temperature:v1");
    const smartctl = candidate("smartctl:storage-health:v1", {
      component: "smartctl",
      usage: "storageHealth",
    });
    const motherboard = candidate("pawnio:motherboard-sensors:v1", {
      usage: "motherboardSensors",
    });

    expect(externalComponentGuidanceCopyKey(pawnio)).toBe(
      "pawnioCpuPackageTemperature",
    );
    expect(externalComponentGuidanceDocsUrl(pawnio)).toContain("#pawnio");
    expect(externalComponentGuidanceCopyKey(smartctl)).toBe(
      "smartctlStorageHealth",
    );
    expect(externalComponentGuidanceDocsUrl(smartctl)).toContain("#smartctl");
    expect(externalComponentGuidanceCopyKey(motherboard)).toBe(
      "pawnioMotherboardSensors",
    );
    expect(externalComponentGuidanceDocsUrl(motherboard)).toContain(
      "#pawnio-motherboard-sensors",
    );
  });

  it("keeps the storage permission instructions distinct from missing setup", () => {
    const missing = candidate("smartctl:storage-health:v1", {
      component: "smartctl",
      usage: "storageHealth",
    });
    const permission = { ...missing, reasonKind: "permission" } as const;

    expect(externalComponentGuidanceActionKey(missing)).toBe(
      "externalComponentGuidance.candidates.smartctlStorageHealth.action",
    );
    expect(externalComponentGuidanceActionKey(permission)).toBe(
      "externalComponentGuidance.candidates.smartctlStorageHealth.permissionAction",
    );
  });

  it("shares sensor service instructions across PawnIO usages and failure kinds", () => {
    for (const usage of [
      "cpuPackageTemperature",
      "motherboardSensors",
    ] as const) {
      for (const reasonKind of [
        "missing",
        "permission",
        "misconfigured",
        "failed",
      ] as const) {
        expect(
          externalComponentGuidanceActionKey(
            candidate(
              usage === "cpuPackageTemperature"
                ? "pawnio:cpu-package-temperature:v1"
                : "pawnio:motherboard-sensors:v1",
              { usage, reasonKind },
            ),
          ),
        ).toBe("sensorService.setupGuidance");
      }
    }
  });

  it("uses Japanese docs URLs for Japanese UI languages", () => {
    const pawnio = candidate("pawnio:cpu-package-temperature:v1");

    expect(externalComponentGuidanceDocsBaseUrl("ja")).toContain(
      "external-components.ja.md",
    );
    expect(externalComponentGuidanceDocsUrl(pawnio, "ja-JP")).toContain(
      "external-components.ja.md#pawnio",
    );
    expect(externalComponentGuidanceDocsBaseUrl("ru")).toContain(
      "external-components.md",
    );
  });
});
