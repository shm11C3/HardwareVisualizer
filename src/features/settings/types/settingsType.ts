import type { sizeOptions } from "@/features/hardware/consts/chart";
import type { ChartDataType } from "../../hardware/types/hardwareDataType";

export type LibreHardwareMonitorImportSettings = {
  enabled: boolean;
  host: string;
  port: number;
  useHttps: boolean;
  refreshInterval: number;
  timeout: number;
  basicAuthUsername?: string;
  basicAuthPassword?: string;
};

export type Settings = {
  language: string;
  theme:
    | "light"
    | "dark"
    | "sky"
    | "grove"
    | "sunset"
    | "nebula"
    | "orbit"
    | "cappuccino"
    | "espresso";
  displayTargets: Array<ChartDataType>;
  graphSize: (typeof sizeOptions)[number];
  lineGraphBorder: boolean;
  lineGraphFill: boolean;
  lineGraphColor: {
    cpu: string;
    memory: string;
    gpu: string;
  };
  lineGraphMix: boolean;
  lineGraphShowLegend: boolean;
  lineGraphShowScale: boolean;
  backgroundImgOpacity: number;
  selectedBackgroundImg: string | null;
  temperatureUnit: "C" | "F";
  libreHardwareMonitorImport: LibreHardwareMonitorImportSettings;
};

export type BackgroundImage = {
  fileId: string;
  imageData: string;
};
