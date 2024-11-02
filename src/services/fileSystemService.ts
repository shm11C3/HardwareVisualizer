import { invoke } from "@tauri-apps/api/core";

export const getBgImage = async (): Promise<string> => {
  return await invoke("get_background_image");
};

export const saveBgImage = async (image: string): Promise<string> => {
  return await invoke("save_background_image", { imageData: image });
};

export const deleteBgImage = async (): Promise<void> => {
  return await invoke("delete_background_image");
};
