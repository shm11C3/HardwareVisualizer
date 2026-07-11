import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GraphSizeSlider } from "./GraphSizeSlider";

const mockUpdateSettingAtom = vi.hoisted(() => vi.fn());
const mockSettings = vi.hoisted(() => ({
  graphFitToWindow: false,
  graphMarginPx: 32,
  graphSize: "xl",
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/components/ui/slider", () => ({
  Slider: ({
    disabled,
    "aria-label": ariaLabel,
  }: {
    disabled?: boolean;
    "aria-label"?: string;
  }) => (
    <input type="range" aria-label={ariaLabel} disabled={disabled} readOnly />
  ),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: mockSettings,
    updateSettingAtom: mockUpdateSettingAtom,
  }),
}));

describe("GraphSizeSlider", () => {
  beforeEach(() => {
    mockSettings.graphFitToWindow = false;
    mockSettings.graphMarginPx = 32;
    mockSettings.graphSize = "xl";
    mockUpdateSettingAtom.mockReset();
    mockUpdateSettingAtom.mockResolvedValue(undefined);
  });

  afterEach(cleanup);

  it("groups fit-to-window and margin controls under graph size", () => {
    render(<GraphSizeSlider />);

    const group = screen.getByRole("group");
    expect(group).toHaveTextContent(
      "pages.settings.customTheme.graphStyle.size",
    );
    expect(group).not.toHaveClass("border");
    const slider = screen.getByRole("slider");
    const marginInput = screen.getByRole("spinbutton");
    const checkbox = screen.getByRole("checkbox");
    const sizeRow = screen.getByTestId("graph-size-row");
    expect(sizeRow).toHaveClass("sm:grid-cols-[minmax(0,1fr)_12rem]");
    expect(sizeRow).toContainElement(slider);
    expect(sizeRow).toContainElement(marginInput);
    expect(
      slider.compareDocumentPosition(checkbox) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(marginInput).toBeDisabled();
    expect(slider).toBeEnabled();
  });

  it("enables fit-to-window from the checkbox", async () => {
    render(<GraphSizeSlider />);

    fireEvent.click(screen.getByRole("checkbox"));

    await waitFor(() => {
      expect(mockUpdateSettingAtom).toHaveBeenCalledWith(
        "graphFitToWindow",
        true,
      );
    });
  });

  it("disables fixed graph sizing while fit-to-window is enabled", () => {
    mockSettings.graphFitToWindow = true;

    render(<GraphSizeSlider />);

    expect(screen.getByRole("checkbox")).toBeChecked();
    expect(screen.getByRole("spinbutton")).toBeEnabled();
    expect(screen.getByRole("slider")).toBeDisabled();
  });
});
