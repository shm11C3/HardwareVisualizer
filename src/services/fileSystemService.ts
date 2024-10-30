import { invoke } from "@tauri-apps/api/core";

export const getBgImage = async (): Promise<string> => {
  return await invoke("get_background_image");
};
