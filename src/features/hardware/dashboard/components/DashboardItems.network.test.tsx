import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

const { initNetwork } = vi.hoisted(() => ({
  initNetwork: vi.fn(() => new Promise<void>(() => undefined)),
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
  it("shows loading independently of an unavailable fallback", () => {
    render(<NetworkInfo />);

    expect(screen.getByTestId("network-info-loading")).toBeVisible();
    expect(initNetwork).toHaveBeenCalledOnce();
  });
});
