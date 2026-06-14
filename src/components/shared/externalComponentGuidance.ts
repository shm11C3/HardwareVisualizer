import type {
  ExternalComponentGuidanceCandidate,
  ExternalComponentGuidanceView,
} from "@/rspc/bindings";
import type { SelectedDisplayType } from "@/types/ui";

export const EXTERNAL_COMPONENT_DOCS_BASE_URL =
  "https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.md";

const EXTERNAL_COMPONENT_DOCS_JA_URL =
  "https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.ja.md";

const EXTERNAL_COMPONENT_DOCS_ANCHORS: Record<string, string> = {
  "pawnio:cpu-package-temperature:v1": "#pawnio",
  "smartctl:storage-health:v1": "#smartctl",
};

type ExternalComponentGuidanceCopyKey =
  | "pawnioCpuPackageTemperature"
  | "smartctlStorageHealth"
  | "generic";

const EXTERNAL_COMPONENT_COPY_KEYS: Record<
  string,
  ExternalComponentGuidanceCopyKey
> = {
  "pawnio:cpu-package-temperature:v1": "pawnioCpuPackageTemperature",
  "smartctl:storage-health:v1": "smartctlStorageHealth",
};

export const externalComponentGuidanceViewForDisplayTarget = (
  displayTarget: SelectedDisplayType | null,
): ExternalComponentGuidanceView | null => {
  switch (displayTarget) {
    case "dashboard":
      return "dashboard";
    case "cpuDetail":
      return "cpuDetail";
    default:
      return null;
  }
};

export const externalComponentGuidanceCopyKey = (
  candidate: ExternalComponentGuidanceCandidate,
): ExternalComponentGuidanceCopyKey =>
  EXTERNAL_COMPONENT_COPY_KEYS[candidate.key] ?? "generic";

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
