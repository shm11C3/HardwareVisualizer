import { renderHook } from "@testing-library/react";
import { Provider } from "jotai";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useTauriStore } from "@/hooks/useTauriStore";

const mockSetVisibleItems = vi.fn();
const mockSetVisibleItemsVersion = vi.fn();
const mockToggleTitleIconVisibility = vi.fn();

const migratedStore = (key: string, defaultValue: unknown) => {
  if (key.endsWith("Version")) {
    return [1, mockSetVisibleItemsVersion, false, false];
  }
  return [defaultValue, mockSetVisibleItems, false, false];
};

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: vi.fn(),
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
    vi.mocked(useTauriStore).mockImplementation(
      migratedStore as unknown as typeof useTauriStore,
    );
  });

  it("does not stamp the migration version when the store failed to load", () => {
    // Both keys settle to their defaults after a failed read. Writing the
    // current version from there would make the real, still-unmigrated list
    // skip its migration on the next launch.
    vi.mocked(useTauriStore).mockImplementation(((
      key: string,
      defaultValue: unknown,
    ) => {
      if (key.endsWith("Version")) {
        return [0, mockSetVisibleItemsVersion, false, true];
      }
      return [defaultValue, mockSetVisibleItems, false, true];
    }) as unknown as typeof useTauriStore);

    renderHook(() => useDashboardSelector(), { wrapper: Provider });

    expect(mockSetVisibleItems).not.toHaveBeenCalled();
    expect(mockSetVisibleItemsVersion).not.toHaveBeenCalled();
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
