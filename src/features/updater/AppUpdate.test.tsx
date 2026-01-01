import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Mock useUpdater
const mockInstall = vi.fn();
const mockUseUpdater = vi.fn();

vi.mock("./hooks/useAppUpdate", () => ({
  useUpdater: () => mockUseUpdater(),
}));

// Mock useTranslation
vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, params?: Record<string, unknown>) => {
      const translations: Record<string, string> = {
        "pages.updater.title": "Update Available",
        "pages.updater.description": `Version ${params?.version || ""} is available`,
        "pages.updater.releaseNotesDescription": `View release notes at ${params?.releaseNotesUrl || ""}`,
        "pages.updater.currentVersion": `Current: ${params?.currentVersion || ""}`,
        "pages.updater.later": "Later",
        "pages.updater.updateAndRestart": "Update and Restart",
        "pages.updater.needRestart": "Please restart to complete the update",
      };
      return translations[key] || key;
    },
  }),
}));

// Mock NeedRestart component
vi.mock("@/components/shared/System", () => ({
  NeedRestart: ({ description }: { description: string }) => (
    <div data-testid="need-restart">{description}</div>
  ),
}));

// Mock UpdateTopBar component
vi.mock("./components/UpdateBar", () => ({
  UpdateTopBar: ({
    percent,
    transferredBytes,
    totalBytes,
  }: {
    percent: number;
    transferredBytes?: bigint;
    totalBytes?: bigint | null;
  }) => (
    <div data-testid="update-top-bar">
      <div>Progress: {percent}%</div>
      {transferredBytes !== undefined && totalBytes !== null && (
        <div>
          {transferredBytes.toString()} / {totalBytes?.toString()}
        </div>
      )}
    </div>
  ),
}));

import { AppUpdate } from "@/features/updater/AppUpdate";

describe("AppUpdate", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUseUpdater.mockReturnValue({
      meta: null,
      installing: false,
      percent: null,
      downloaded: 0n,
      total: null,
      install: mockInstall,
      isFinished: false,
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("should not display modal when meta is null", () => {
    render(<AppUpdate />);
    expect(screen.queryByText("Update Available")).not.toBeInTheDocument();
  });

  it("should display update modal when meta exists", () => {
    mockUseUpdater.mockReturnValue({
      meta: {
        version: "2.0.0",
        currentVersion: "1.0.0",
        notes: "Release notes",
        pubDate: "2025-12-31",
      },
      installing: false,
      percent: null,
      downloaded: 0n,
      total: null,
      install: mockInstall,
      isFinished: false,
    });

    render(<AppUpdate />);

    expect(screen.getByText("Update Available")).toBeInTheDocument();
    expect(screen.getByText("Version 2.0.0 is available")).toBeInTheDocument();
    expect(screen.getByText("Current: 1.0.0")).toBeInTheDocument();
  });

  it("should display UpdateTopBar when installing and percent is not null", () => {
    mockUseUpdater.mockReturnValue({
      meta: null,
      installing: true,
      percent: 50,
      downloaded: 50n,
      total: 100n,
      install: mockInstall,
      isFinished: false,
    });

    render(<AppUpdate />);

    expect(screen.getByTestId("update-top-bar")).toBeInTheDocument();
    expect(screen.getByText("Progress: 50%")).toBeInTheDocument();
    expect(screen.getByText("50 / 100")).toBeInTheDocument();
  });

  it("should display restart prompt when isFinished is true", () => {
    mockUseUpdater.mockReturnValue({
      meta: null,
      installing: false,
      percent: null,
      downloaded: 0n,
      total: null,
      install: mockInstall,
      isFinished: true,
    });

    render(<AppUpdate />);

    expect(screen.getByTestId("need-restart")).toBeInTheDocument();
    expect(
      screen.getByText("Please restart to complete the update"),
    ).toBeInTheDocument();
  });

  it("should close modal when cancel button is clicked", async () => {
    const user = userEvent.setup();

    mockUseUpdater.mockReturnValue({
      meta: {
        version: "2.0.0",
        currentVersion: "1.0.0",
        notes: null,
        pubDate: null,
      },
      installing: false,
      percent: null,
      downloaded: 0n,
      total: null,
      install: mockInstall,
      isFinished: false,
    });

    render(<AppUpdate />);

    expect(screen.getByText("Update Available")).toBeInTheDocument();

    const laterButton = screen.getByRole("button", { name: "Later" });
    await user.click(laterButton);

    expect(mockInstall).not.toHaveBeenCalled();
    expect(screen.queryByText("Update Available")).not.toBeInTheDocument();
  });

  it("should call install when update button is clicked", async () => {
    const user = userEvent.setup();

    mockUseUpdater.mockReturnValue({
      meta: {
        version: "2.0.0",
        currentVersion: "1.0.0",
        notes: null,
        pubDate: null,
      },
      installing: false,
      percent: null,
      downloaded: 0n,
      total: null,
      install: mockInstall,
      isFinished: false,
    });

    render(<AppUpdate />);

    const updateButton = screen.getByRole("button", {
      name: "Update and Restart",
    });
    await user.click(updateButton);

    expect(mockInstall).toHaveBeenCalledTimes(1);
  });
});
