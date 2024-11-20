import type { NameValues } from "@/types/hardwareDataType";
import { invoke } from "@tauri-apps/api/core";

// [TODO] bindings に置き換える
export const getGpuUsage = async (): Promise<number> => {
  return await invoke("get_gpu_usage");
};

// [TODO] bindings に置き換える
export const getGpuTemperature = async (): Promise<NameValues> => {
  return await invoke("get_gpu_temperature");
};

// [TODO] bindings に置き換える
export const getGpuFanSpeed = async (): Promise<NameValues> => {
  return await invoke("get_nvidia_gpu_cooler");
};
