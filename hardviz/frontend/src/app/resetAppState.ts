import { clearTauriStore } from "@/lib/tauriStore";

/**
 * Recovery path of the inner error boundary: clear the persisted Tauri Store,
 * then restart the in-memory app state. The restart runs even when clearing
 * fails, because the render error may come from in-memory state alone.
 */
export const resetAppState = async (restartAppState: () => void) => {
  try {
    await clearTauriStore();
  } catch (error) {
    console.error("Failed to reset app state:", error);
  }
  restartAppState();
};
