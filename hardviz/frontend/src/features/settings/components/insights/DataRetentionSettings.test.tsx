import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { DataRetentionSettings } from "./DataRetentionSettings";

const mocks = vi.hoisted(() => ({
  settings: {
    hardwareArchive: { retentionDays: 30, scheduledDataDeletion: true },
    storageHealth: { retentionDays: 365 as number | null },
  },
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/rspc/bindings", () => ({
  commands: { restartApp: vi.fn() },
}));

vi.mock("@/components/SettingWriteFailure", () => ({
  SettingWriteFailure: () => null,
}));

vi.mock("@/hooks/settings/useRestartRequired", () => ({
  useRestartRequired: () => ({
    isRestartRequired: false,
    markRestartRequired: vi.fn(),
  }),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: mocks.settings,
    setHardwareArchiveRetentionDays: vi.fn(),
    setScheduledDataDeletion: vi.fn(),
    setStorageHealthRetentionDays: vi.fn(),
  }),
}));

describe("DataRetentionSettings", () => {
  beforeEach(() => {
    mocks.settings.storageHealth.retentionDays = 365;
  });

  afterEach(cleanup);

  it("replaces an unsaved storage health draft when the stored retention changes", () => {
    const { rerender } = render(<DataRetentionSettings />);

    const input = screen.getByPlaceholderText(
      "pages.settings.insights.storageHealth.retention.placeHolder",
    );
    fireEvent.change(input, { target: { value: "9999" } });
    expect(input).toHaveValue(9999);

    mocks.settings.storageHealth.retentionDays = 730;
    rerender(<DataRetentionSettings />);

    expect(input).toHaveValue(730);
  });
});
