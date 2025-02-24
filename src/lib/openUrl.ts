import { open } from "@tauri-apps/plugin-shell";

export const openURL = async (url: string) => {
  if (!isValidURL(url)) {
    throw new Error("Invalid URL");
  }

  await open(url);
};

const isValidURL = (url: string) => {
  try {
    new URL(url);
    return true;
  } catch {
    return false;
  }
};
