import { renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriStore } from "@/hooks/useTauriStore";

const mockSetVisibleItems = vi.fn();
const mockSetVisibleItemsVersion = vi.fn();
const mockToggleTitleIconVisibility = vi.fn();

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi
    .fn()
    .mockImplementation((key: string, defaultValue: unknown) => {
      if (key.endsWith("Version")) {
        return [1, mockSetVisibleItemsVersion, false];
      }
      return [defaultValue, mockSetVisibleItems, false];
    }),
}));

vi.mock("@/hooks/useTitleIconVisualSelector", () => ({
  useTitleIconVisualSelector: () => ({
    toggleTitleIconVisibility: mockToggleTitleIconVisibility,
  }),
}));

import { useDashboardSelector } from "./useDashboardSelector";

describe("useDashboardSelector", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("isolates specifications visibility and does not change the Classic title", () => {
    renderHook(
      () =>
        useDashboardSelector({
          visibleItemsKey: "systemSpecificationsVisibleItems",
          visibleItemsVersionKey: "systemSpecificationsVisibleItemsVersion",
          syncDashboardTitleVisibility: false,
        }),
      { wrapper: Provider },
    );

    expect(useTauriStore).toHaveBeenCalledWith(
      "systemSpecificationsVisibleItems",
      expect.any(Array),
    );
    expect(useTauriStore).toHaveBeenCalledWith(
      "systemSpecificationsVisibleItemsVersion",
      0,
    );
    expect(mockToggleTitleIconVisibility).not.toHaveBeenCalled();
  });
});
