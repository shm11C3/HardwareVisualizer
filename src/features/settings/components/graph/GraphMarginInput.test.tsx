import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { GraphMarginInput } from "./GraphMarginInput";

const mockUpdateSettingAtom = vi.hoisted(() => vi.fn());
const mockSettings = vi.hoisted(() => ({
  graphFitToWindow: false,
  graphMarginPx: 32,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

vi.mock("@/features/settings/hooks/useSettingsAtom", () => ({
  useSettingsAtom: () => ({
    settings: mockSettings,
    updateSettingAtom: mockUpdateSettingAtom,
  }),
}));

describe("GraphMarginInput", () => {
  beforeEach(() => {
    mockSettings.graphFitToWindow = false;
    mockSettings.graphMarginPx = 32;
    mockUpdateSettingAtom.mockReset();
    mockUpdateSettingAtom.mockResolvedValue(undefined);
  });

  afterEach(cleanup);

  it("is only active while fit-to-window is enabled", () => {
    const { rerender } = render(<GraphMarginInput />);
    expect(screen.getByRole("spinbutton")).toBeDisabled();

    mockSettings.graphFitToWindow = true;
    rerender(<GraphMarginInput />);
    expect(screen.getByRole("spinbutton")).toBeEnabled();
  });

  it("clamps and persists the entered margin on blur", async () => {
    mockSettings.graphFitToWindow = true;
    render(<GraphMarginInput />);

    const input = screen.getByRole("spinbutton");
    fireEvent.change(input, { target: { value: "250" } });
    fireEvent.blur(input);

    await waitFor(() => {
      expect(mockUpdateSettingAtom).toHaveBeenCalledWith("graphMarginPx", 200);
    });
  });
});
