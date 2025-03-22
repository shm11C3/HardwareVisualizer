import { type Store, load } from "@tauri-apps/plugin-store";

let storeInstance: Store | null = null;

export const getStoreInstance = async () => {
  if (!storeInstance) {
    storeInstance = await load("store.json", { autoSave: true });
  }
  return storeInstance;
};
