import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createStore, Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import type { ProductAnalyticsClientSettings } from "@/rspc/bindings";
import { settingsAtom } from "@/store/settings";
import { ProductAnalyticsSection } from "./ProductAnalyticsSection";

const mocks = vi.hoisted(() => ({
  openURL: vi.fn(),
  setProductAnalyticsConsent: vi.fn(),
}));

vi.mock("@/lib/openUrl", () => ({ openURL: mocks.openURL }));
vi.mock("@/rspc/bindings", () => ({
  commands: {
    setProductAnalyticsConsent: mocks.setProductAnalyticsConsent,
  },
}));

const renderSection = (productAnalytics: ProductAnalyticsClientSettings) => {
  const store = createStore();
  store.set(settingsAtom, (prev) => ({ ...prev, productAnalytics }));
  render(
    <Provider store={store}>
      <ProductAnalyticsSection />
    </Provider>,
  );
  return store;
};

const toggle = () => screen.getByRole("switch", { name: "Product analytics" });

describe("ProductAnalyticsSection", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.setProductAnalyticsConsent.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.openURL.mockResolvedValue(undefined);
  });

  it("shows the switch on and the contract version only for an effective grant", () => {
    renderSection({
      consent: "granted",
      consentVersion: 1,
      currentConsentVersion: 1,
    });

    expect(toggle()).toBeChecked();
    expect(
      screen.getByText("Sharing aggregate usage under contract v1"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/The data contract changed/)).toBeNull();
  });

  it.each<[string, ProductAnalyticsClientSettings]>([
    [
      "undecided",
      { consent: "undecided", consentVersion: 0, currentConsentVersion: 1 },
    ],
    [
      "declined",
      { consent: "declined", consentVersion: 1, currentConsentVersion: 1 },
    ],
  ])("shows the switch off and no sharing while %s", (_label, state) => {
    renderSection(state);

    expect(toggle()).not.toBeChecked();
    expect(screen.getByText("Not sharing")).toBeInTheDocument();
  });

  it("treats a grant under an older contract as off and explains why", () => {
    renderSection({
      consent: "granted",
      consentVersion: 1,
      currentConsentVersion: 2,
    });

    expect(toggle()).not.toBeChecked();
    expect(screen.getByText("Not sharing")).toBeInTheDocument();
    expect(
      screen.getByText(/The data contract changed to v2/),
    ).toBeInTheDocument();
  });

  it("turning the switch on grants, and off declines", async () => {
    const user = userEvent.setup();
    const store = renderSection({
      consent: "declined",
      consentVersion: 1,
      currentConsentVersion: 1,
    });

    await user.click(toggle());
    await waitFor(() => expect(toggle()).toBeChecked());
    expect(mocks.setProductAnalyticsConsent).toHaveBeenLastCalledWith(
      "granted",
    );

    await user.click(toggle());
    await waitFor(() => expect(toggle()).not.toBeChecked());
    expect(mocks.setProductAnalyticsConsent).toHaveBeenLastCalledWith(
      "declined",
    );
    expect(store.get(settingsAtom).productAnalytics.consent).toBe("declined");
  });

  it("keeps the saved state and renders the write failure when the save fails", async () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    mocks.setProductAnalyticsConsent.mockResolvedValue({
      status: "error",
      error: "disk full",
    });
    const user = userEvent.setup();
    renderSection({
      consent: "declined",
      consentVersion: 1,
      currentConsentVersion: 1,
    });

    await user.click(toggle());

    expect(await screen.findByTestId("action-failure")).toBeInTheDocument();
    expect(toggle()).not.toBeChecked();
    expect(toggle()).toBeEnabled();
    consoleError.mockRestore();
  });

  it("opens each document and reports a link that cannot be opened", async () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    const user = userEvent.setup();
    renderSection({
      consent: "declined",
      consentVersion: 1,
      currentConsentVersion: 1,
    });

    await user.click(screen.getByRole("button", { name: "Privacy notice" }));
    expect(mocks.openURL).toHaveBeenCalledWith(
      expect.stringContaining("product-analytics.md"),
    );
    expect(screen.queryByTestId("action-failure")).toBeNull();

    mocks.openURL.mockRejectedValue(new Error("blocked"));
    await user.click(screen.getByRole("button", { name: "Service source" }));
    expect(await screen.findByText("Could not open the link.")).toBeVisible();
    consoleError.mockRestore();
  });
});
