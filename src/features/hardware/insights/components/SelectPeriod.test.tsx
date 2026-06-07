import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SelectPeriod } from "./SelectPeriod";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

vi.mock("@/components/ui/select", () => ({
  Select: ({
    children,
    onValueChange,
    value,
  }: {
    children: ReactNode;
    onValueChange: (value: string) => void;
    value: string;
  }) => (
    <select
      aria-label="Period"
      onChange={(event) => onValueChange(event.currentTarget.value)}
      value={value}
    >
      {children}
    </select>
  ),
  SelectContent: ({ children }: { children: ReactNode }) => children,
  SelectItem: ({ children, value }: { children: ReactNode; value: string }) => (
    <option value={value}>{children}</option>
  ),
  SelectTrigger: () => null,
  SelectValue: () => null,
}));

describe("SelectPeriod", () => {
  afterEach(() => {
    cleanup();
  });

  it("returns the selected period as a number", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();

    render(
      <SelectPeriod
        options={[
          { label: "1 hour", value: 60 },
          { label: "3 hours", value: 180 },
        ]}
        selected={60}
        onChange={onChange}
      />,
    );

    await user.selectOptions(screen.getByRole("combobox"), "180");

    expect(onChange).toHaveBeenCalledWith(180);
    expect(typeof onChange.mock.calls[0][0]).toBe("number");
  });

  it("ignores the non-period placeholder option", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();

    render(
      <SelectPeriod
        options={[{ label: "1 hour", value: 60 }]}
        selected={null}
        onChange={onChange}
        showDefaultOption
      />,
    );

    await user.selectOptions(screen.getByRole("combobox"), "not-selected");

    expect(onChange).not.toHaveBeenCalled();
  });
});
