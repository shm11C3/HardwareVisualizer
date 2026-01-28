import { load, type Store } from "@tauri-apps/plugin-store";

let storeInstance: Store | null = null;

export const getStoreInstance = async () => {
  if (!storeInstance) {
    storeInstance = await load("store.json", { autoSave: true, defaults: {} });
  }
  return storeInstance;
};

export const clearTauriStore = async () => {
  const store = await getStoreInstance();
  await store.reset();
};
