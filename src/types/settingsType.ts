import type { sizeOptions } from "@/consts/chart";
import type { ChartDataType } from "./hardwareDataType";

export type Settings = {
  language: string;
  theme: "light" | "dark";
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
};

export type BackgroundImage = {
  fileId: string;
  imageData: string;
};
