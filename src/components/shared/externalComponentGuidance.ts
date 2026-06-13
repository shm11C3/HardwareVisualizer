import type {
  ExternalComponentGuidanceCandidate,
  ExternalComponentGuidanceView,
} from "@/rspc/bindings";
import type { SelectedDisplayType } from "@/types/ui";

const EXTERNAL_COMPONENT_DOCS_BASE_URL =
  "https://github.com/shm11C3/HardwareVisualizer/blob/develop/docs/user/external-components.md";

const EXTERNAL_COMPONENT_DOCS_URLS: Record<string, string> = {
  "pawnio:cpu-package-temperature:v1": `${EXTERNAL_COMPONENT_DOCS_BASE_URL}#pawnio`,
  "smartctl:storage-health:v1": `${EXTERNAL_COMPONENT_DOCS_BASE_URL}#smartctl`,
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

export const externalComponentGuidanceDocsUrl = (
  candidate: ExternalComponentGuidanceCandidate,
) =>
  EXTERNAL_COMPONENT_DOCS_URLS[candidate.key] ??
  EXTERNAL_COMPONENT_DOCS_BASE_URL;
