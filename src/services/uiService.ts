import { invoke } from "@tauri-apps/api/core";

export const toggleDecoration = async (isDecorated: boolean): Promise<void> => {
  return await invoke("set_decoration", { isDecorated });
};
