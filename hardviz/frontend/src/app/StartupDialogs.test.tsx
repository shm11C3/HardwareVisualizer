import { act, cleanup, render, screen } from "@testing-library/react";
import { Provider } from "jotai";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import type { ProductAnalyticsClientSettings } from "@/rspc/bindings";
import type { SelectedDisplayType } from "@/types/ui";
import { StartupDialogs } from "./StartupDialogs";

// The four higher-priority flows are stubs driven from the test through this
// store, so each can be requested or pending on demand. Only the consent
// dialog under test is the real component.
const flows = vi.hoisted(() => {
  type Flow = { requested: boolean; pending: boolean };
  const names = ["close", "guidance", "conversion", "nsis"] as const;
  const state = {} as Record<(typeof names)[number], Flow>;
  const listeners = new Set<() => void>();
  const stubFor = async (
    name: "close" | "guidance" | "conversion" | "nsis",
  ) => {
    const { useLayoutEffect, useSyncExternalStore } = await import("react");
    return function flowStub({
      onRequestChange,
      onPendingChange,
    }: {
      onRequestChange?: (requested: boolean) => void;
      onPendingChange?: (pending: boolean) => void;
    }) {
      const flow = useSyncExternalStore(
        (listener) => {
          listeners.add(listener);
          return () => listeners.delete(listener);
        },
        () => state[name],
      );
      useLayoutEffect(
        () => onRequestChange?.(flow.requested),
        [flow.requested, onRequestChange],
      );
      useLayoutEffect(
        () => onPendingChange?.(flow.pending),
        [flow.pending, onPendingChange],
      );
      return null;
    };
  };

  return {
    stubFor,
    state,
    listeners,
    reset: () => {
      for (const name of names)
        state[name] = { requested: false, pending: false };
    },
    set: (name: (typeof names)[number], next: Partial<Flow>) => {
      state[name] = { ...state[name], ...next };
      for (const listener of [...listeners]) listener();
    },
  };
});

vi.mock("./CloseToTrayFirstRunDialog", async () => ({
  CloseToTrayFirstRunDialog: await flows.stubFor("close"),
}));
vi.mock("./ExternalComponentGuidanceDialog", async () => ({
  ExternalComponentGuidanceDialog: await flows.stubFor("guidance"),
}));
vi.mock("./DatabaseConversionPromptDialog", async () => ({
  DatabaseConversionPromptDialog: await flows.stubFor("conversion"),
}));
vi.mock("./NsisMigrationNoticeDialog", async () => ({
  NsisMigrationNoticeDialog: await flows.stubFor("nsis"),
}));
vi.mock("@/hooks/tauri/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: vi.fn() }),
}));
vi.mock("@/lib/openUrl", () => ({ openURL: vi.fn() }));
vi.mock("@/rspc/bindings", () => ({
  commands: { setProductAnalyticsConsent: vi.fn() },
}));

const undecided: ProductAnalyticsClientSettings = {
  consent: "undecided",
  consentVersion: 0,
  currentConsentVersion: 1,
};
const declined: ProductAnalyticsClientSettings = {
  consent: "declined",
  consentVersion: 0,
  currentConsentVersion: 1,
};
const staleGrant: ProductAnalyticsClientSettings = {
  consent: "granted",
  consentVersion: 1,
  currentConsentVersion: 2,
};

const Harness = ({
  settingsLoaded = true,
  displayTarget = "dashboard",
  productAnalytics = undecided,
}: {
  settingsLoaded?: boolean;
  displayTarget?: SelectedDisplayType | null;
  productAnalytics?: ProductAnalyticsClientSettings;
}) => (
  <Provider>
    <StartupDialogs
      settingsLoaded={settingsLoaded}
      settingsSettled
      displayTarget={displayTarget}
      nsisMigrationNoticeDismissed={false}
      productAnalytics={productAnalytics}
    />
  </Provider>
);

const consentDialog = () =>
  screen.queryByRole("alertdialog", {
    name: "Share anonymous usage statistics?",
  });
const setFlow = (...args: Parameters<typeof flows.set>) =>
  act(() => flows.set(...args));

describe("StartupDialogs consent arbitration", () => {
  beforeEach(() => {
    flows.reset();
  });
  afterEach(() => {
    cleanup();
  });

  it("opens the consent dialog when every other flow is idle", () => {
    render(<Harness />);
    expect(consentDialog()).toBeInTheDocument();
  });

  it.each(["guidance", "conversion", "nsis"] as const)(
    "waits while the %s flow could still request the slot",
    (name) => {
      flows.set(name, { pending: true });
      render(<Harness />);
      expect(consentDialog()).toBeNull();

      setFlow(name, { pending: false });
      expect(consentDialog()).toBeInTheDocument();
    },
  );

  it.each(["close", "guidance", "conversion", "nsis"] as const)(
    "waits while the %s flow has a request open",
    (name) => {
      flows.set(name, { requested: true });
      render(<Harness />);
      expect(consentDialog()).toBeNull();

      setFlow(name, { requested: false });
      expect(consentDialog()).toBeInTheDocument();
    },
  );

  it("waits for settings to load", () => {
    const { rerender } = render(<Harness settingsLoaded={false} />);
    expect(consentDialog()).toBeNull();

    rerender(<Harness settingsLoaded />);
    expect(consentDialog()).toBeInTheDocument();
  });

  it("is skipped for the launch when the window is on Settings at the free moment, and never reappears", () => {
    const { rerender } = render(<Harness displayTarget="settings" />);
    expect(consentDialog()).toBeNull();

    rerender(<Harness displayTarget="dashboard" />);
    expect(consentDialog()).toBeNull();

    // Even a later free moment does not re-offer it within this launch.
    setFlow("guidance", { requested: true });
    setFlow("guidance", { requested: false });
    expect(consentDialog()).toBeNull();
  });

  it("does not re-evaluate after the free moment passed", () => {
    // Free at launch with a granted-and-current consent: skipped. A later
    // change to the stored consent must not open the dialog mid-session.
    const granted = {
      ...undecided,
      consent: "granted" as const,
      consentVersion: 1,
    };
    const { rerender } = render(<Harness productAnalytics={granted} />);
    rerender(<Harness productAnalytics={undecided} />);
    expect(consentDialog()).toBeNull();
  });

  it("never asks a declined installation, whatever its contract version", () => {
    render(<Harness productAnalytics={declined} />);
    expect(consentDialog()).toBeNull();
  });

  it("asks a granted installation again after a contract change", () => {
    render(<Harness productAnalytics={staleGrant} />);
    expect(consentDialog()).toBeInTheDocument();
  });

  it("keeps the consent dialog open when another flow requests the slot later", () => {
    render(<Harness />);
    expect(consentDialog()).toBeInTheDocument();

    setFlow("guidance", { requested: true });

    // The owner keeps the slot until its own flow finishes; a late request
    // never covers it.
    expect(consentDialog()).toBeInTheDocument();
  });
});
