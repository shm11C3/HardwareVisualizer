import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const mockSetStoredTab = vi.fn();
let mockStoredTab = "performance";
let mockIsPending = false;

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      ({
        "pages.dashboard.name": "Dashboard",
        "pages.dashboard.viewTabs": "Dashboard views",
        "pages.dashboard.tabs.performance": "Performance",
        "pages.dashboard.tabs.systemSpecifications": "System Specifications",
      })[key] ?? key,
  }),
}));

vi.mock("@/components/shared/BurnInShift", () => ({
  BurnInShift: ({ children }: { children: ReactNode }) => children,
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: () => [
    mockStoredTab,
    async (value: string) => {
      mockSetStoredTab(value);
      mockStoredTab = value;
    },
    mockIsPending,
  ],
}));

vi.mock("./Dashboard", () => ({
  Dashboard: () => <div data-testid="system-specifications-content" />,
}));

vi.mock("../performance/Performance", () => ({
  Performance: () => <div data-testid="performance-content" />,
}));

import {
  GroupedDashboard,
  normalizeGroupedDashboardTab,
} from "./GroupedDashboard";

describe("GroupedDashboard", () => {
  afterEach(cleanup);

  beforeEach(() => {
    vi.clearAllMocks();
    mockStoredTab = "performance";
    mockIsPending = false;
  });

  it("defaults invalid UI-local state to Performance", async () => {
    mockStoredTab = "removed-tab";

    render(<GroupedDashboard />);

    expect(screen.getByRole("tab", { name: "Performance" })).toHaveAttribute(
      "data-state",
      "active",
    );
    expect(
      await screen.findByTestId("performance-content"),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("system-specifications-content"),
    ).not.toBeInTheDocument();
    expect(normalizeGroupedDashboardTab("removed-tab")).toBe("performance");
  });

  it("persists the selected tab and unmounts inactive content", async () => {
    const user = userEvent.setup();
    const { rerender } = render(<GroupedDashboard />);

    await user.click(
      screen.getByRole("tab", { name: "System Specifications" }),
    );
    expect(mockSetStoredTab).toHaveBeenCalledWith("systemSpecifications");

    rerender(<GroupedDashboard />);

    expect(
      screen.getByRole("tab", { name: "System Specifications" }),
    ).toHaveAttribute("data-state", "active");
    expect(
      await screen.findByTestId("system-specifications-content"),
    ).toBeVisible();
    expect(screen.queryByTestId("performance-content")).not.toBeInTheDocument();
  });

  it("waits for the UI-local tab value before mounting either view", () => {
    mockIsPending = true;

    render(<GroupedDashboard />);

    expect(screen.queryByTestId("grouped-dashboard")).not.toBeInTheDocument();
    expect(screen.queryByTestId("performance-content")).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("system-specifications-content"),
    ).not.toBeInTheDocument();
  });
});
