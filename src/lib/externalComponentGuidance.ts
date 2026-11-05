import type { ExternalComponentGuidanceCandidate } from "@/rspc/bindings";

export const EXTERNAL_COMPONENT_DOCS_BASE_URL =
  "https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.md";

const EXTERNAL_COMPONENT_DOCS_JA_URL =
  "https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.ja.md";

const EXTERNAL_COMPONENT_DOCS_ANCHORS: Record<string, string> = {
  "pawnio:cpu-package-temperature:v1": "#pawnio",
  "pawnio:motherboard-sensors:v1": "#pawnio-motherboard-sensors",
  "smartctl:storage-health:v1": "#smartctl",
};

type ExternalComponentGuidanceCopyKey =
  | "pawnioCpuPackageTemperature"
  | "pawnioMotherboardSensors"
  | "smartctlStorageHealth"
  | "generic";

const EXTERNAL_COMPONENT_COPY_KEYS: Record<
  string,
  ExternalComponentGuidanceCopyKey
> = {
  "pawnio:cpu-package-temperature:v1": "pawnioCpuPackageTemperature",
  "pawnio:motherboard-sensors:v1": "pawnioMotherboardSensors",
  "smartctl:storage-health:v1": "smartctlStorageHealth",
};

/** Visibility reported by the mounted Performance view after layout hydration. */
export type PerformanceGuidanceContext = {
  settled: boolean;
  cpuTemperatureVisible: boolean;
  motherboardSensorsVisible: boolean;
  expanded: boolean;
};

export const externalComponentGuidanceCopyKey = (
  candidate: Pick<ExternalComponentGuidanceCandidate, "key">,
): ExternalComponentGuidanceCopyKey =>
  EXTERNAL_COMPONENT_COPY_KEYS[candidate.key] ?? "generic";

export const externalComponentGuidanceActionKey = (
  candidate: ExternalComponentGuidanceCandidate,
) => {
  if (candidate.component === "pawnio") return "sensorService.setupGuidance";

  const action =
    candidate.reasonKind === "permission" ? "permissionAction" : "action";
  const copyKey = externalComponentGuidanceCopyKey(candidate);
  const actionCopy = copyKey === "smartctlStorageHealth" ? copyKey : "generic";
  return `externalComponentGuidance.candidates.${actionCopy}.${action}` as const;
};

export const externalComponentGuidanceDocsBaseUrl = (
  language?: string | null,
) =>
  language?.toLowerCase().startsWith("ja")
    ? EXTERNAL_COMPONENT_DOCS_JA_URL
    : EXTERNAL_COMPONENT_DOCS_BASE_URL;

export const externalComponentGuidanceDocsUrl = (
  candidate: ExternalComponentGuidanceCandidate,
  language?: string | null,
) =>
  `${externalComponentGuidanceDocsBaseUrl(language)}${
    EXTERNAL_COMPONENT_DOCS_ANCHORS[candidate.key] ?? ""
  }`;
