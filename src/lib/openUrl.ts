import { open } from "@tauri-apps/plugin-shell";

export const openURL = async (url: string) => {
  if (!isValidURL(url)) console.error("Invalid URL:", url);

  try {
    await open(url);
  } catch (error) {
    console.error("Failed to open URL:", error);
  }
};

const isValidURL = (url: string) => {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
};
