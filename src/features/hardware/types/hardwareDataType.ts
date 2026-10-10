export type NameValues = Array<{
  name: string;
  value: number;
}>;

export type FanSpeedStatus = "active" | "inactive" | "invalid";

export type MotherboardTemperatureValues = Array<{
  name: string;
  value: number;
  source: string;
}>;

export type MotherboardFanSpeedValues = Array<{
  name: string;
  rpm: number | null;
  status: FanSpeedStatus;
  source: string;
}>;

export type DataStats = "avg" | "max" | "min";
