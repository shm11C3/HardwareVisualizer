import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useSettingsAtom } from "@/hooks/settings/useSettingsAtom";
import { RetentionNote } from "./RetentionNote";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { days?: number }) =>
      `${key}:${options?.days ?? ""}`,
  }),
}));

vi.mock("@/hooks/settings/useSettingsAtom", () => ({
  useSettingsAtom: vi.fn(),
}));

const withArchive = (hardwareArchive: {
  scheduledDataDeletion?: boolean;
  retentionDays?: number;
}) =>
  vi.mocked(useSettingsAtom).mockReturnValue({
    settings: { hardwareArchive },
  } as ReturnType<typeof useSettingsAtom>);

describe("RetentionNote", () => {
  afterEach(() => {
    cleanup();
  });

  it("names the Retention Period when the selected range reaches past it", () => {
    withArchive({ scheduledDataDeletion: true, retentionDays: 30 });

    render(<RetentionNote periods={[60, 525600]} />);

    expect(screen.getByTestId("insight-retention-note")).toHaveTextContent(
      "pages.insights.retentionNote:30",
    );
  });

  it("stays hidden while the Retention Period covers every selected range", () => {
    withArchive({ scheduledDataDeletion: true, retentionDays: 365 });

    render(<RetentionNote periods={[43200, 525600]} />);

    expect(screen.queryByTestId("insight-retention-note")).toBeNull();
  });

  it("ignores charts that have no period selected yet", () => {
    withArchive({ scheduledDataDeletion: true, retentionDays: 30 });

    render(<RetentionNote periods={[null, 43200]} />);

    expect(screen.queryByTestId("insight-retention-note")).toBeNull();
  });
});
