import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import type {
  ExternalComponentSetupResult,
  ExternalComponentSetupStatus,
} from "@/rspc/bindings";
import { ExternalComponentSetupSection } from "./ExternalComponentSetupSection";

const mocks = vi.hoisted(() => ({
  error: vi.fn(),
  getExternalComponentSetupComponents: vi.fn(),
  getExternalComponentSetupStatus: vi.fn(),
  platform: vi.fn(() => "windows"),
  useElevationAvailability: vi.fn((): string | null => "available"),
  useProcessElevated: vi.fn((): boolean | null => false),
  restartApp: vi.fn(),
  runExternalComponentSetup: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-os", () => ({
  platform: mocks.platform,
}));

vi.mock("@/hooks/useTauriDialog", () => ({
  useTauriDialog: () => ({
    error: mocks.error,
  }),
}));

vi.mock("@/hooks/useElevationAvailability", async (importOriginal) => ({
  ...(await importOriginal<
    typeof import("@/hooks/useElevationAvailability")
  >()),
  useElevationAvailability: mocks.useElevationAvailability,
  useProcessElevated: mocks.useProcessElevated,
}));

vi.mock("@/rspc/bindings", () => ({
  commands: {
    getExternalComponentSetupComponents:
      mocks.getExternalComponentSetupComponents,
    getExternalComponentSetupStatus: mocks.getExternalComponentSetupStatus,
    restartApp: mocks.restartApp,
    runExternalComponentSetup: mocks.runExternalComponentSetup,
  },
}));

const status = (
  overrides: Partial<ExternalComponentSetupStatus> = {},
): ExternalComponentSetupStatus => ({
  component: "pawnio",
  support: "supported",
  runtime: {
    state: "notInstalled",
    version: null,
    installLocation: null,
    detail: null,
  },
  moduleFiles: [
    { fileName: "IntelMSR.bin", condition: "missing" },
    { fileName: "RyzenSMU.bin", condition: "current" },
    { fileName: "AMDFamily17.bin", condition: "missing" },
    { fileName: "LpcIO.bin", condition: "missing" },
  ],
  pinnedRuntimeVersion: "2.2.0",
  pinnedModulesVersion: "0.2.11",
  complete: false,
  setupBlocker: null,
  ...overrides,
});

const completeStatus = (): ExternalComponentSetupStatus =>
  status({
    runtime: {
      state: "installed",
      version: "2.2.0",
      installLocation: "C:\\Program Files\\PawnIO",
      detail: null,
    },
    moduleFiles: status().moduleFiles.map((file) => ({
      ...file,
      condition: "current",
    })),
    complete: true,
  });

const outdatedStatus = (): ExternalComponentSetupStatus => {
  const outdated = completeStatus();
  outdated.moduleFiles[0] = { fileName: "IntelMSR.bin", condition: "outdated" };
  outdated.moduleFiles[3] = { fileName: "LpcIO.bin", condition: "outdated" };
  return { ...outdated, complete: false };
};

const result = (
  overrides: Partial<ExternalComponentSetupResult> = {},
): ExternalComponentSetupResult => ({
  component: "pawnio",
  outcome: "installed",
  failureStage: null,
  detail: null,
  status: completeStatus(),
  ...overrides,
});

describe("ExternalComponentSetupSection", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.platform.mockReturnValue("windows");
    mocks.useElevationAvailability.mockReturnValue("available");
    mocks.getExternalComponentSetupComponents.mockResolvedValue(["pawnio"]);
    mocks.getExternalComponentSetupStatus.mockResolvedValue({
      status: "ok",
      data: status(),
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("renders nothing outside Windows", () => {
    mocks.platform.mockReturnValue("macos");

    const { container } = render(<ExternalComponentSetupSection />);

    expect(container).toBeEmptyDOMElement();
    expect(mocks.getExternalComponentSetupComponents).not.toHaveBeenCalled();
  });

  it("shows the runtime and module file state with an install action", async () => {
    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByText(
        "Runtime not installed (setup installs version 2.2.0)",
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Module files: 1 of 4 present.", { exact: false }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Missing: IntelMSR.bin, AMDFamily17.bin, LpcIO.bin"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Install" })).toBeEnabled();
  });

  it("offers to update outdated files when nothing is missing", async () => {
    mocks.getExternalComponentSetupStatus.mockResolvedValue({
      status: "ok",
      data: outdatedStatus(),
    });

    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByText("Module files: 4 of 4 present.", {
        exact: false,
      }),
    ).toBeInTheDocument();
    expect(
      screen.getByText("Update available (0.2.11): IntelMSR.bin, LpcIO.bin"),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Missing:/)).toBeNull();
    expect(screen.getByRole("button", { name: "Update files" })).toBeEnabled();
  });

  it("names both steps when files are missing and outdated", async () => {
    const withMissing = outdatedStatus();
    withMissing.moduleFiles[1] = {
      fileName: "RyzenSMU.bin",
      condition: "missing",
    };
    mocks.getExternalComponentSetupStatus.mockResolvedValue({
      status: "ok",
      data: withMissing,
    });

    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByRole("button", { name: "Install and update files" }),
    ).toBeEnabled();
    expect(screen.getByText("Missing: RyzenSMU.bin")).toBeInTheDocument();
  });

  it("counts an unrecognized file as present and does not offer to replace it", async () => {
    const unrecognized = completeStatus();
    unrecognized.moduleFiles[0] = {
      fileName: "IntelMSR.bin",
      condition: "unrecognized",
    };
    mocks.getExternalComponentSetupStatus.mockResolvedValue({
      status: "ok",
      data: unrecognized,
    });

    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByText("Module files: 4 of 4 present.", {
        exact: false,
      }),
    ).toBeInTheDocument();
    expect(screen.queryByText(/Update available/)).toBeNull();
    expect(screen.getByRole("button", { name: "Installed" })).toBeDisabled();
  });

  it("reports an update instead of an installation", async () => {
    const user = userEvent.setup();
    mocks.getExternalComponentSetupStatus.mockResolvedValue({
      status: "ok",
      data: outdatedStatus(),
    });
    mocks.runExternalComponentSetup.mockResolvedValue({
      status: "ok",
      data: result(),
    });

    render(<ExternalComponentSetupSection />);

    await user.click(
      await screen.findByRole("button", { name: "Update files" }),
    );

    expect(
      await screen.findByText(
        "Restart HardwareVisualizer to start using the updated module files.",
      ),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(
        "Restart HardwareVisualizer to start using the newly installed component.",
      ),
    ).toBeNull();
    expect(
      screen.getByText(
        "PawnIO module files were updated. Restart HardwareVisualizer to use them.",
      ),
    ).toBeInTheDocument();
    expect(mocks.error).not.toHaveBeenCalled();
  });

  it("disables setup outside Program Files and says why", async () => {
    mocks.useElevationAvailability.mockReturnValue("unprotectedLocation");

    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByRole("button", { name: "Install" }),
    ).toBeDisabled();
    expect(
      screen.getByText(/not installed under Program Files/),
    ).toBeInTheDocument();
    expect(mocks.runExternalComponentSetup).not.toHaveBeenCalled();
  });

  it("keeps setup disabled until elevation is positively available", async () => {
    mocks.useElevationAvailability.mockReturnValue(null);

    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByRole("button", { name: "Install" }),
    ).toBeDisabled();
  });

  it("does not blame the install folder when availability is unknown", async () => {
    mocks.useElevationAvailability.mockReturnValue("unknown");

    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByRole("button", { name: "Install" }),
    ).toBeDisabled();
    expect(screen.getByText(/could not verify/)).toBeInTheDocument();
    expect(screen.queryByText(/not installed under Program Files/)).toBeNull();
  });

  it("disables the action when setup has nothing left to do", async () => {
    mocks.getExternalComponentSetupStatus.mockResolvedValue({
      status: "ok",
      data: completeStatus(),
    });

    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByText("Runtime installed (version 2.2.0)"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Installed" })).toBeDisabled();
  });

  it("blocks setup while the state is uncertain instead of assuming absence", async () => {
    mocks.getExternalComponentSetupStatus.mockResolvedValue({
      status: "ok",
      data: status({
        runtime: {
          state: "unknown",
          version: null,
          installLocation: null,
          detail: "RegOpenKeyExW failed with 5",
        },
        setupBlocker: "runtime state is unknown: RegOpenKeyExW failed with 5",
      }),
    });

    render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByText("The runtime state could not be determined."),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        "Setup is unavailable until the state can be read: runtime state is unknown: RegOpenKeyExW failed with 5",
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Install" })).toBeDisabled();
  });

  it("runs setup, refreshes the state, and asks for a restart on success", async () => {
    const user = userEvent.setup();
    mocks.runExternalComponentSetup.mockResolvedValue({
      status: "ok",
      data: result(),
    });

    render(<ExternalComponentSetupSection />);

    await user.click(await screen.findByRole("button", { name: "Install" }));

    await waitFor(() => {
      expect(mocks.runExternalComponentSetup).toHaveBeenCalledWith("pawnio");
    });
    expect(
      await screen.findByText(
        "Restart HardwareVisualizer to start using the newly installed component.",
      ),
    ).toBeInTheDocument();
    // The restart dialog hides the page from the accessibility tree while open.
    expect(
      screen.getByText("Runtime installed (version 2.2.0)"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Installed", hidden: true }),
    ).toBeDisabled();
    expect(
      screen.getByText(
        "PawnIO was installed. Restart HardwareVisualizer to use the new sensors.",
      ),
    ).toBeInTheDocument();
    expect(mocks.error).not.toHaveBeenCalled();
  });

  it("reports a cancelled elevation prompt without an error dialog", async () => {
    const user = userEvent.setup();
    mocks.runExternalComponentSetup.mockResolvedValue({
      status: "ok",
      data: result({ outcome: "cancelled", status: status() }),
    });

    render(<ExternalComponentSetupSection />);

    await user.click(await screen.findByRole("button", { name: "Install" }));

    expect(
      await screen.findByText(
        "Installation was cancelled at the administrator prompt.",
      ),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Install" })).toBeEnabled();
    expect(mocks.error).not.toHaveBeenCalled();
  });

  it("explains a failure by its stage in an error dialog", async () => {
    const user = userEvent.setup();
    mocks.runExternalComponentSetup.mockResolvedValue({
      status: "ok",
      data: result({
        outcome: "failed",
        failureStage: "verifyRuntime",
        detail: "the setup process exited with code 13",
        status: status(),
      }),
    });

    render(<ExternalComponentSetupSection />);

    await user.click(await screen.findByRole("button", { name: "Install" }));

    await waitFor(() => {
      expect(mocks.error).toHaveBeenCalledWith(
        "Installation failed: The downloaded runtime installer did not match its expected digest. (the setup process exited with code 13)",
      );
    });
  });

  it("explains a timed-out setup as a retryable failure", async () => {
    const user = userEvent.setup();
    mocks.runExternalComponentSetup.mockResolvedValue({
      status: "ok",
      data: result({
        outcome: "failed",
        failureStage: "setupTimedOut",
        detail: "the setup process did not finish in time and was stopped",
        status: status(),
      }),
    });

    render(<ExternalComponentSetupSection />);

    await user.click(await screen.findByRole("button", { name: "Install" }));

    await waitFor(() => {
      expect(mocks.error).toHaveBeenCalledWith(
        "Installation failed: Setup did not finish in time and was stopped. (the setup process did not finish in time and was stopped)",
      );
    });
    expect(screen.getByRole("button", { name: "Install" })).toBeEnabled();
  });

  it("tells the user to restart the app when a timed-out setup may still be running", async () => {
    const user = userEvent.setup();
    mocks.runExternalComponentSetup.mockResolvedValue({
      status: "ok",
      data: result({
        outcome: "failed",
        failureStage: "setupStillRunning",
        detail:
          "the setup process did not finish in time and may still be running; restart the app before trying again",
        status: status(),
      }),
    });

    render(<ExternalComponentSetupSection />);

    await user.click(await screen.findByRole("button", { name: "Install" }));

    await waitFor(() => {
      expect(mocks.error).toHaveBeenCalledWith(
        "Installation failed: Setup did not finish in time and may still be running. Check Task Manager and confirm PawnIO_setup.exe has finished or stopped, then restart the app before trying again. (the setup process did not finish in time and may still be running; restart the app before trying again)",
      );
    });
  });

  it("surfaces a command error as a failure", async () => {
    const user = userEvent.setup();
    mocks.runExternalComponentSetup.mockResolvedValue({
      status: "error",
      error: "External Component Setup for pawnio is already running",
    });

    render(<ExternalComponentSetupSection />);

    await user.click(await screen.findByRole("button", { name: "Install" }));

    await waitFor(() => {
      expect(mocks.error).toHaveBeenCalledWith(
        "Installation failed: External Component Setup for pawnio is already running",
      );
    });
  });

  it("surfaces a thrown setup call as a failure and clears the spinner", async () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    const user = userEvent.setup();
    mocks.runExternalComponentSetup.mockRejectedValue(
      new Error("IPC channel closed"),
    );

    render(<ExternalComponentSetupSection />);

    await user.click(await screen.findByRole("button", { name: "Install" }));

    await waitFor(() => {
      expect(mocks.error).toHaveBeenCalledWith(
        "Installation failed: IPC channel closed",
      );
    });
    expect(screen.getByRole("button", { name: "Install" })).toBeEnabled();
    expect(screen.queryByRole("status")).toBeNull();
    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });

  it("shows the state error instead of the skeleton when the status call throws", async () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    mocks.getExternalComponentSetupStatus.mockRejectedValue(
      new Error("IPC channel closed"),
    );

    const { container } = render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByText("The component state could not be read."),
    ).toBeInTheDocument();
    expect(container.querySelector(".animate-pulse")).toBeNull();
    expect(screen.queryByRole("button", { name: "Install" })).toBeNull();
    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });

  it("shows the state error instead of the skeleton when the component list throws", async () => {
    const consoleError = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});
    mocks.getExternalComponentSetupComponents.mockRejectedValue(
      new Error("IPC channel closed"),
    );

    const { container } = render(<ExternalComponentSetupSection />);

    expect(
      await screen.findByText("The component state could not be read."),
    ).toBeInTheDocument();
    expect(container.querySelector(".animate-pulse")).toBeNull();
    expect(mocks.getExternalComponentSetupStatus).not.toHaveBeenCalled();
    expect(consoleError).toHaveBeenCalled();
    consoleError.mockRestore();
  });
});
