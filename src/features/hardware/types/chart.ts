export type DataArchive = {
  id: number;
  cpu_avg: number | null;
  cpu_max: number | null;
  cpu_min: number | null;
  ram_avg: number | null;
  ram_max: number | null;
  ram_min: number | null;
  timestamp: number;
};

export type GpuDataArchive = {
  id: number;
  gpu_name: string;
  usage_avg: number | null;
  usage_max: number | null;
  usage_min: number | null;
  temperature_avg: number | null;
  temperature_max: number | null;
  temperature_min: number | null;
  dedicated_memory_avg: number | null;
  dedicated_memory_max: number | null;
  dedicated_memory_min: number | null;
  timestamp: number;
};

export type SingleDataArchive = {
  id: number;
  value: number | null;
  timestamp: number;
};
