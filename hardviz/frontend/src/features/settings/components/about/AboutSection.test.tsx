import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "@/lib/i18n";
import { AboutSection } from "./AboutSection";

const mocks = vi.hoisted(() => ({
  getVersion: vi.fn(),
  openURL: vi.fn(),
}));

vi.mock("@tauri-apps/api/app", () => ({ getVersion: mocks.getVersion }));
vi.mock("@tauri-apps/plugin-os", () => ({ platform: () => "windows" }));
vi.mock("@/lib/openUrl", () => ({ openURL: mocks.openURL }));
vi.mock("@/components/HardwareCompatibilityReport", () => ({
  HardwareCompatibilityReport: () => null,
}));

const surveyButton = () =>
  screen.getByRole("button", { name: /Answer the Survey/ });

describe("AboutSection survey button", () => {
  afterEach(() => {
    cleanup();
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mocks.getVersion.mockResolvedValue("1.12.1");
    mocks.openURL.mockResolvedValue(undefined);
  });

  it("opens the survey with the app version even before the label loads", async () => {
    let resolveFirst: (v: string) => void = () => {};
    mocks.getVersion
      .mockReturnValueOnce(
        new Promise<string>((resolve) => {
          resolveFirst = resolve;
        }),
      )
      .mockResolvedValue("1.12.1");
    render(<AboutSection onShowLicense={() => {}} />);

    await userEvent.click(surveyButton());

    await waitFor(() =>
      expect(mocks.openURL).toHaveBeenCalledWith(
        "https://hardviz.com/survey/?source=app&app_version=1.12.1&os=windows",
      ),
    );
    resolveFirst("1.12.1");
  });

  it("shows a failure under the controls when the survey cannot open", async () => {
    mocks.openURL.mockRejectedValueOnce(new Error("blocked"));
    render(<AboutSection onShowLicense={() => {}} />);

    await userEvent.click(surveyButton());
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "Could not open the survey.",
    );

    await userEvent.click(surveyButton());
    await waitFor(() => expect(screen.queryByRole("alert")).toBeNull());
    expect(mocks.openURL).toHaveBeenCalledTimes(2);
  });
});
