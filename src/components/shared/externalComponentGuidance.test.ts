import { describe, expect, it } from "vitest";
import type { ExternalComponentGuidanceCandidate } from "@/rspc/bindings";
import {
  externalComponentGuidanceCopyKey,
  externalComponentGuidanceDocsBaseUrl,
  externalComponentGuidanceDocsUrl,
  externalComponentGuidanceViewForDisplayTarget,
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
  it("maps visible app screens to guidance views", () => {
    expect(externalComponentGuidanceViewForDisplayTarget("dashboard")).toBe(
      "dashboard",
    );
    expect(externalComponentGuidanceViewForDisplayTarget("cpuDetail")).toBe(
      "cpuDetail",
    );
    expect(
      externalComponentGuidanceViewForDisplayTarget("settings"),
    ).toBeNull();
  });

  it("maps stable candidate keys to copy keys and docs URLs", () => {
    const pawnio = candidate("pawnio:cpu-package-temperature:v1");
    const smartctl = candidate("smartctl:storage-health:v1", {
      component: "smartctl",
      usage: "storageHealth",
    });

    expect(externalComponentGuidanceCopyKey(pawnio)).toBe(
      "pawnioCpuPackageTemperature",
    );
    expect(externalComponentGuidanceDocsUrl(pawnio)).toContain("#pawnio");
    expect(externalComponentGuidanceCopyKey(smartctl)).toBe(
      "smartctlStorageHealth",
    );
    expect(externalComponentGuidanceDocsUrl(smartctl)).toContain("#smartctl");
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
