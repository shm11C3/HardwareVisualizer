import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type React from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { LicensePage } from "@/features/settings/components/LicensePage";

const mocks = vi.hoisted(() => ({
  readLicenseFile: vi.fn(),
  readThirdPartyNoticesFile: vi.fn(),
  openLicenseFilePath: vi.fn(),
}));

// A failed read is the panel's failure state, never an OS dialog.
vi.mock("@/hooks/tauri/useTauriDialog", () => {
  throw new Error("LicensePage must not open a dialog");
});

vi.mock("@/rspc/bindings", () => ({
  commands: {
    readLicenseFile: mocks.readLicenseFile,
    readThirdPartyNoticesFile: mocks.readThirdPartyNoticesFile,
    openLicenseFilePath: mocks.openLicenseFilePath,
  },
}));

// Radix ScrollArea needs ResizeObserver, which jsdom does not provide.
vi.mock("@/components/ui/scroll-area", () => ({
  ScrollArea: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
}));

describe("LicensePage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.spyOn(console, "error").mockImplementation(() => {});
    mocks.readThirdPartyNoticesFile.mockResolvedValue({
      status: "ok",
      data: "third party text",
    });
    mocks.openLicenseFilePath.mockResolvedValue({ status: "ok", data: null });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it("renders the failure state with retry, then the text once the retry succeeds", async () => {
    const user = userEvent.setup();
    mocks.readLicenseFile
      .mockResolvedValueOnce({ status: "error", error: "ENOENT: LICENSE" })
      .mockResolvedValueOnce({ status: "ok", data: "license text" });

    render(<LicensePage onBack={() => {}} />);

    const failure = await screen.findByTestId("load-failure");
    expect(failure).toHaveTextContent("Could not load the license text.");
    expect(failure).not.toHaveTextContent("ENOENT");

    await user.click(screen.getByRole("button", { name: "Try again" }));

    expect(await screen.findByText("license text")).toBeInTheDocument();
    expect(screen.queryByTestId("load-failure")).toBeNull();
  });

  it("keeps a successful read visible when the other read fails", async () => {
    const user = userEvent.setup();
    mocks.readLicenseFile.mockResolvedValue({
      status: "ok",
      data: "license body",
    });
    mocks.readThirdPartyNoticesFile.mockRejectedValue(new Error("ipc closed"));

    render(<LicensePage onBack={() => {}} />);

    expect(await screen.findByText("license body")).toBeInTheDocument();
    expect(screen.queryByTestId("load-failure")).toBeNull();

    await user.click(screen.getByRole("tab", { name: "Third-Party Licenses" }));

    expect(await screen.findByTestId("load-failure")).toHaveTextContent(
      "Could not load the license text.",
    );
  });

  it("reports a failed open under the button, not as a dialog", async () => {
    const user = userEvent.setup();
    mocks.readLicenseFile.mockResolvedValue({
      status: "ok",
      data: "license body",
    });
    mocks.openLicenseFilePath.mockResolvedValue({
      status: "error",
      error: "no file manager",
    });

    render(<LicensePage onBack={() => {}} />);
    await screen.findByText("license body");

    await user.click(screen.getAllByRole("button", { name: /explorer/i })[0]);

    await waitFor(() =>
      expect(screen.getByTestId("action-failure")).toHaveTextContent(
        "Could not open the folder that contains the license files.",
      ),
    );
  });

  it("tracks a failed open per tab: another tab neither shows nor clears it", async () => {
    const user = userEvent.setup();
    mocks.readLicenseFile.mockResolvedValue({
      status: "ok",
      data: "license body",
    });
    mocks.openLicenseFilePath
      .mockResolvedValueOnce({ status: "error", error: "no file manager" })
      .mockResolvedValueOnce({ status: "ok", data: null });

    render(<LicensePage onBack={() => {}} />);
    await screen.findByText("license body");

    await user.click(screen.getByRole("button", { name: /explorer/i }));
    expect(await screen.findByTestId("action-failure")).toBeInTheDocument();

    // The other tab's button has not failed, so it shows no line ...
    await user.click(screen.getByRole("tab", { name: "Third-Party Licenses" }));
    await screen.findByText("third party text");
    expect(screen.queryByTestId("action-failure")).toBeNull();

    // ... and its success does not clear the first tab's failure.
    await user.click(screen.getByRole("button", { name: /explorer/i }));
    await waitFor(() =>
      expect(mocks.openLicenseFilePath).toHaveBeenCalledTimes(2),
    );
    expect(screen.queryByTestId("action-failure")).toBeNull();

    await user.click(screen.getByRole("tab", { name: "Application License" }));
    expect(await screen.findByTestId("action-failure")).toHaveTextContent(
      "Could not open the folder that contains the license files.",
    );
  });
});
