import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createStore, Provider } from "jotai";
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("./hooks/useMenu", () => ({
  useMenu: () => ({
    isOpen: true,
    displayTarget: "dashboard",
    handleMenuClick: vi.fn(),
    toggleMenu: vi.fn(),
  }),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) =>
      key === "pages.updater.available"
        ? `Update available: v${params?.["version"]}`
        : key,
  }),
}));

import { SideMenu } from "@/features/menu/SideMenu";
import {
  availableUpdateAtom,
  updateDialogOpenAtom,
  updateInstallStartedAtom,
} from "@/store/appUpdate";

const renderSideMenu = (store: ReturnType<typeof createStore>) =>
  render(
    <Provider store={store}>
      <SideMenu
        isFullScreen={false}
        navigationLayout="grouped"
        settingsLoaded
        onPrefetchScreen={vi.fn()}
      />
    </Provider>,
  );

const storeWithUpdate = () => {
  const store = createStore();
  store.set(availableUpdateAtom, {
    version: "2.0.0",
    currentVersion: "1.0.0",
    notes: null,
    pubDate: null,
  });
  return store;
};

describe("SideMenu update entry", () => {
  afterEach(() => {
    cleanup();
  });

  it("is absent while no update is known", () => {
    renderSideMenu(createStore());

    expect(screen.queryByText(/Update available/)).not.toBeInTheDocument();
  });

  it("opens the update dialog when pressed", async () => {
    const store = storeWithUpdate();
    renderSideMenu(store);

    await userEvent.click(screen.getByText("Update available: v2.0.0"));

    expect(store.get(updateDialogOpenAtom)).toBe(true);
  });

  it("is hidden once an install has started", () => {
    const store = storeWithUpdate();
    store.set(updateInstallStartedAtom, true);
    renderSideMenu(store);

    expect(screen.queryByText(/Update available/)).not.toBeInTheDocument();
  });
});
