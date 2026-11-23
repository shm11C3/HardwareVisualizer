import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  initNetwork: vi.fn().mockResolvedValue(undefined),
  networkLoadFailed: false,
}));
const { initNetwork } = mocks;

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    networkInfo: [],
    networkLoadFailed: mocks.networkLoadFailed,
    initNetwork: mocks.initNetwork,
  }),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      selectedBackgroundImg: null,
      backgroundImgOpacity: 100,
    },
  }),
}));

import { NetworkInfo } from "./DashboardItems";

describe("NetworkInfo", () => {
  afterEach(() => {
    cleanup();
    mocks.networkLoadFailed = false;
    vi.clearAllMocks();
  });

  it("shows an explicit unavailable state in System Specifications", async () => {
    render(<NetworkInfo showUnavailableState />);

    expect(screen.getByTestId("network-info-loading")).toBeVisible();
    expect(
      await screen.findByText(
        "pages.dashboard.systemSpecifications.networkUnavailable",
      ),
    ).toBeVisible();
    expect(initNetwork).toHaveBeenCalledOnce();
  });

  it("shows a failed network read with retry where the classic Dashboard mounts it (no unavailable copy)", () => {
    mocks.networkLoadFailed = true;

    render(<NetworkInfo />);

    expect(screen.getByTestId("load-failure")).toHaveTextContent(
      "pages.dashboard.systemSpecifications.networkLoadFailed",
    );
    initNetwork.mockClear();
    fireEvent.click(screen.getByRole("button", { name: "shared.retry" }));
    expect(initNetwork).toHaveBeenCalledOnce();
    expect(
      screen.queryByText(
        "pages.dashboard.systemSpecifications.networkUnavailable",
      ),
    ).toBeNull();
  });
});
