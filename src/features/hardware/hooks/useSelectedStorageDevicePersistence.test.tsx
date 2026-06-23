import { act, renderHook } from "@testing-library/react";
import { Provider, useAtom } from "jotai";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { selectedStorageDeviceIdAtom } from "@/features/hardware/store/chart";
import { useSelectedStorageDevicePersistence } from "./useSelectedStorageDevicePersistence";

const mocks = vi.hoisted(() => ({
  storeValue: null as string | null,
  setStored: vi.fn(),
  isPending: false,
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: () => [mocks.storeValue, mocks.setStored, mocks.isPending],
}));

const wrapper = ({ children }: { children: ReactNode }) => (
  <Provider>{children}</Provider>
);

const useHarness = () => {
  useSelectedStorageDevicePersistence();
  const [selected, setSelected] = useAtom(selectedStorageDeviceIdAtom);
  return { selected, setSelected };
};

describe("useSelectedStorageDevicePersistence", () => {
  beforeEach(() => {
    mocks.storeValue = null;
    mocks.isPending = false;
    mocks.setStored.mockClear();
  });

  it("restores the persisted storage device selection on mount", () => {
    mocks.storeValue = "disk-b";

    const { result } = renderHook(() => useHarness(), { wrapper });

    expect(result.current.selected).toBe("disk-b");
  });

  it("persists the selection when it changes", () => {
    const { result } = renderHook(() => useHarness(), { wrapper });

    act(() => {
      result.current.setSelected("disk-c");
    });

    expect(mocks.setStored).toHaveBeenCalledWith("disk-c");
  });

  it("does not restore while the store is still loading", () => {
    mocks.storeValue = "disk-b";
    mocks.isPending = true;

    const { result } = renderHook(() => useHarness(), { wrapper });

    expect(result.current.selected).toBeNull();
  });
});
