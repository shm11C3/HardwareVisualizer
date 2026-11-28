import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createStore, Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import type { ProductAnalyticsClientSettings } from "@/rspc/bindings";
import { settingsAtom } from "@/store/settings";
import { ProductAnalyticsConsentDialog } from "./ProductAnalyticsConsentDialog";

const mocks = vi.hoisted(() => ({
  error: vi.fn(),
  openURL: vi.fn(),
  setProductAnalyticsConsent: vi.fn(),
  setProductAnalyticsPromptOpen: vi.fn(),
}));

vi.mock("@/hooks/tauri/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: mocks.error }),
}));
vi.mock("@/lib/openUrl", () => ({ openURL: mocks.openURL }));
vi.mock("@/rspc/bindings", () => ({
  commands: {
    setProductAnalyticsConsent: mocks.setProductAnalyticsConsent,
    setProductAnalyticsPromptOpen: mocks.setProductAnalyticsPromptOpen,
  },
}));

const undecided: ProductAnalyticsClientSettings = {
  consent: "undecided",
  consentVersion: 0,
  currentConsentVersion: 1,
};

const renderDialog = (
  props: Partial<Parameters<typeof ProductAnalyticsConsentDialog>[0]> = {},
) => {
  const store = createStore();
  store.set(settingsAtom, (prev) => ({ ...prev, productAnalytics: undecided }));
  const onRequestChange = vi.fn();
  render(
    <Provider store={store}>
      <ProductAnalyticsConsentDialog
        arbitrationFree
        displayTarget="dashboard"
        productAnalytics={undecided}
        onRequestChange={onRequestChange}
        {...props}
      />
    </Provider>,
  );
  return { store, onRequestChange };
};

const share = () => screen.getByRole("button", { name: "Share" });
const decline = () => screen.getByRole("button", { name: "Don't share" });

describe("ProductAnalyticsConsentDialog", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.setProductAnalyticsConsent.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.setProductAnalyticsPromptOpen.mockResolvedValue(undefined);
    mocks.openURL.mockResolvedValue(undefined);
    mocks.error.mockResolvedValue(undefined);
  });

  it("tells the backend it is unanswered so quitting the app declines", async () => {
    renderDialog();
    await screen.findByRole("alertdialog");
    await waitFor(() =>
      expect(mocks.setProductAnalyticsPromptOpen).toHaveBeenLastCalledWith(
        true,
      ),
    );

    // Once answered, a quit while the choice is saved must not decline it:
    // the dialog reports itself closed before the grant is written.
    await userEvent.click(share());
    await waitFor(() =>
      expect(mocks.setProductAnalyticsConsent).toHaveBeenCalled(),
    );
    const { calls, invocationCallOrder } =
      mocks.setProductAnalyticsPromptOpen.mock;
    const firstClosed = calls.findIndex(([open]) => open === false);
    expect(firstClosed).toBeGreaterThanOrEqual(0);
    expect(invocationCallOrder[firstClosed]).toBeLessThan(
      mocks.setProductAnalyticsConsent.mock.invocationCallOrder[0] ?? 0,
    );
  });

  it("gives Share and Don't share the same weight and no default focus", async () => {
    renderDialog();
    const dialog = await screen.findByRole("alertdialog");

    // Neither choice may look recommended: same variant and size means the
    // same class list.
    expect(share().className).toBe(decline().className);
    expect(dialog.contains(document.activeElement)).toBe(false);
    expect(document.activeElement).not.toBe(share());
    expect(document.activeElement).not.toBe(decline());
    // The only way to close it is a choice: no X button.
    expect(screen.queryByRole("button", { name: /close/i })).toBeNull();
  });

  it("scrolls inside the dialog so the choices stay reachable in a compact window", async () => {
    renderDialog();
    expect(await screen.findByRole("alertdialog")).toHaveClass(
      "max-h-[calc(100dvh-2rem)]",
      "overflow-y-auto",
    );
  });

  it("names what is sent and what is never sent, with the four documents", async () => {
    renderDialog();
    await screen.findByRole("alertdialog");

    expect(screen.getByText(/Daily counts of which screens/)).toBeVisible();
    expect(screen.getByText(/Never sent: hardware readings/)).toBeVisible();
    for (const name of [
      "Data contract",
      "Privacy notice",
      "Client source",
      "Service source",
    ]) {
      expect(screen.getByRole("button", { name })).toBeVisible();
    }
  });

  it("opens a document through the shared URL opener", async () => {
    renderDialog();
    await screen.findByRole("alertdialog");

    await userEvent.click(
      screen.getByRole("button", { name: "Client source" }),
    );

    expect(mocks.openURL).toHaveBeenCalledWith(
      expect.stringContaining("hardviz/app/src/analytics"),
    );
  });

  it("grants consent at the current contract version and closes for the session", async () => {
    const { store, onRequestChange } = renderDialog();
    await screen.findByRole("alertdialog");

    await userEvent.click(share());

    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(mocks.setProductAnalyticsConsent).toHaveBeenCalledWith("granted");
    expect(store.get(settingsAtom).productAnalytics).toMatchObject({
      consent: "granted",
      consentVersion: 1,
    });
    expect(onRequestChange).toHaveBeenLastCalledWith(false);
  });

  it("counts Don't share as a decline", async () => {
    const { store } = renderDialog();
    await screen.findByRole("alertdialog");

    await userEvent.click(decline());

    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
    expect(mocks.setProductAnalyticsConsent).toHaveBeenCalledWith("declined");
    expect(store.get(settingsAtom).productAnalytics.consent).toBe("declined");
  });

  it("counts Escape as a decline instead of leaving the choice open", async () => {
    renderDialog();
    await screen.findByRole("alertdialog");

    await userEvent.keyboard("{Escape}");

    await waitFor(() =>
      expect(mocks.setProductAnalyticsConsent).toHaveBeenCalledWith("declined"),
    );
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });

  it("disables both choices while the choice is being saved", async () => {
    let finish: (value: unknown) => void = () => {};
    mocks.setProductAnalyticsConsent.mockReturnValue(
      new Promise((resolve) => {
        finish = resolve;
      }),
    );
    renderDialog();
    await screen.findByRole("alertdialog");

    await userEvent.click(share());

    expect(share()).toBeDisabled();
    expect(decline()).toBeDisabled();
    finish({ status: "ok", data: null });
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });

  it.each([
    [
      "a failed write",
      () =>
        mocks.setProductAnalyticsConsent.mockResolvedValue({
          status: "error",
          error: "disk",
        }),
    ],
    [
      "a rejected call",
      () =>
        mocks.setProductAnalyticsConsent.mockRejectedValue(new Error("ipc")),
    ],
  ])(
    "fails closed on %s: reports it, shares nothing, and closes",
    async (_label, arrange) => {
      const consoleError = vi
        .spyOn(console, "error")
        .mockImplementation(() => {});
      arrange();
      const { store } = renderDialog();
      await screen.findByRole("alertdialog");

      await userEvent.click(share());

      await waitFor(() => expect(mocks.error).toHaveBeenCalledTimes(1));
      expect(mocks.error.mock.calls[0]?.[0]).toMatch(/Nothing is shared/);
      await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
      expect(store.get(settingsAtom).productAnalytics.consent).toBe(
        "undecided",
      );
      consoleError.mockRestore();
    },
  );

  it("stays closed while the arbitration is not free", () => {
    renderDialog({ arbitrationFree: false });
    expect(screen.queryByRole("alertdialog")).toBeNull();
  });
});
