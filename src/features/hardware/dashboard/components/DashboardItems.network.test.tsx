import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const { initNetwork } = vi.hoisted(() => ({
  initNetwork: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    networkInfo: [],
    initNetwork,
  }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: {
      selectedBackgroundImg: null,
      backgroundImgOpacity: 100,
    },
  }),
}));

import { NetworkInfo } from "./DashboardItems";

describe("NetworkInfo", () => {
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
});
