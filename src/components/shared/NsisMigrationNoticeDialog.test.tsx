import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import {
  NSIS_MIGRATION_DOWNLOAD_URL,
  NsisMigrationNoticeDialog,
} from "./NsisMigrationNoticeDialog";

const mocks = vi.hoisted(() => ({
  dismissNsisMigrationNotice: vi.fn(),
  error: vi.fn(),
  getBundleType: vi.fn(),
  openURL: vi.fn(),
}));

vi.mock("@tauri-apps/api/app", () => ({
  BundleType: { Nsis: "nsis", Msi: "msi" },
  getBundleType: mocks.getBundleType,
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({ error: mocks.error }),
}));

vi.mock("@/lib/openUrl", () => ({
  openURL: mocks.openURL,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    dismissNsisMigrationNotice: mocks.dismissNsisMigrationNotice,
  },
}));

const title = "Switch to the MSI installer";

describe("NsisMigrationNoticeDialog", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getBundleType.mockResolvedValue("nsis");
    mocks.dismissNsisMigrationNotice.mockResolvedValue({
      status: "ok",
      data: null,
    });
    mocks.openURL.mockResolvedValue(undefined);
  });

  it("shows the notice only in the NSIS build", async () => {
    render(<NsisMigrationNoticeDialog dismissed={false} />);
    expect(await screen.findByText(title)).toBeInTheDocument();
    expect(
      screen.getByText(/Leave "Delete the application data" unchecked/),
    ).toBeInTheDocument();
  });

  it.each(["msi", null])("stays hidden for bundle type %s", async (type) => {
    mocks.getBundleType.mockResolvedValue(type);
    render(<NsisMigrationNoticeDialog dismissed={false} />);
    await waitFor(() => expect(mocks.getBundleType).toHaveBeenCalled());
    expect(screen.queryByText(title)).not.toBeInTheDocument();
  });

  it("does not query the bundle type once dismissed or before settings load", () => {
    render(<NsisMigrationNoticeDialog dismissed />);
    render(
      <NsisMigrationNoticeDialog dismissed={false} settingsLoaded={false} />,
    );
    expect(mocks.getBundleType).not.toHaveBeenCalled();
    expect(screen.queryByText(title)).not.toBeInTheDocument();
  });

  it("opens the download page", async () => {
    const user = userEvent.setup();
    render(<NsisMigrationNoticeDialog dismissed={false} />);
    await user.click(
      await screen.findByRole("button", { name: /Open download page/ }),
    );
    expect(mocks.openURL).toHaveBeenCalledWith(NSIS_MIGRATION_DOWNLOAD_URL);
  });

  it("hides for this session without persisting", async () => {
    const user = userEvent.setup();
    render(<NsisMigrationNoticeDialog dismissed={false} />);
    await user.click(await screen.findByRole("button", { name: /Hide/ }));
    await user.click(
      await screen.findByRole("menuitem", { name: "Remind me next time" }),
    );
    await waitFor(() =>
      expect(screen.queryByText(title)).not.toBeInTheDocument(),
    );
    expect(mocks.dismissNsisMigrationNotice).not.toHaveBeenCalled();
  });

  it("persists Never show again", async () => {
    const user = userEvent.setup();
    render(<NsisMigrationNoticeDialog dismissed={false} />);
    await user.click(await screen.findByRole("button", { name: /Hide/ }));
    await user.click(
      await screen.findByRole("menuitem", { name: "Never show again" }),
    );
    await waitFor(() =>
      expect(screen.queryByText(title)).not.toBeInTheDocument(),
    );
    expect(mocks.dismissNsisMigrationNotice).toHaveBeenCalledTimes(1);
  });

  it("keeps the notice and reports an error when dismissal fails", async () => {
    mocks.dismissNsisMigrationNotice.mockResolvedValue({
      status: "error",
      error: "write failed",
    });
    const user = userEvent.setup();
    render(<NsisMigrationNoticeDialog dismissed={false} />);
    await user.click(await screen.findByRole("button", { name: /Hide/ }));
    await user.click(
      await screen.findByRole("menuitem", { name: "Never show again" }),
    );
    await waitFor(() =>
      expect(mocks.error).toHaveBeenCalledWith(
        "Failed to save this preference.",
      ),
    );
    expect(screen.getByText(title)).toBeInTheDocument();
  });
});
