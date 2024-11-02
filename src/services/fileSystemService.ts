import type { BackgroundImage } from "@/types/settingsType";
import { invoke } from "@tauri-apps/api/core";

export const getBgImage = async (fileId: string): Promise<string> => {
  return await invoke("get_background_image", { fileId });
};

export const saveBgImage = async (image: string): Promise<string> => {
  return await invoke("save_background_image", { imageData: image });
};

export const deleteBgImage = async (): Promise<void> => {
  return await invoke("delete_background_image");
};

export const fetchBackgroundImages = async (): Promise<
  Array<BackgroundImage>
> => {
  return await invoke("get_background_images");
};
