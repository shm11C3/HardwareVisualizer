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

export type GpuDataArchive = {
  id: number;
  gpu_name: string;
  usage_avg: number;
  usage_max: number;
  usage_min: number;
  temperature_avg: number;
  temperature_max: number;
  temperature_min: number;
  dedicated_memory_avg: number;
  dedicated_memory_max: number;
  dedicated_memory_min: number;
  timestamp: number;
};

export type SingleDataArchive = {
  id: number;
  value: number;
  timestamp: number;
};
