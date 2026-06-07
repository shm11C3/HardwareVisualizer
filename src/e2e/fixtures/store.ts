/**
 * Initial contents of the mocked Tauri store (`store.json`).
 * Keys not listed here fall back to the defaults each `useTauriStore`
 * caller provides (and are then written into the in-memory store).
 */
export const storeFixture: Record<string, unknown> = {
  window_decorated: true,
  sideMenuOpen: false,
  display: "dashboard",
};
