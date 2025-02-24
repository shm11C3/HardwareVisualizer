export type DataArchive = {
  id: number;
  cpu_avg: number;
  cpu_max: number;
  cpu_min: number;
  ram_avg: number;
  ram_max: number;
  ram_min: number;
  timestamp: number;
};

export type ShowDataType =
  | "cpu_avg"
  | "cpu_max"
  | "cpu_min"
  | "ram_avg"
  | "ram_max"
  | "ram_min";
