import { act, renderHook } from "@testing-library/react";
import { Provider, useAtom } from "jotai";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { selectedGpuIdAtom } from "@/features/hardware/store/chart";
import { useSelectedGpuPersistence } from "./useSelectedGpuPersistence";

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
  useSelectedGpuPersistence();
  const [selected, setSelected] = useAtom(selectedGpuIdAtom);
  return { selected, setSelected };
};

describe("useSelectedGpuPersistence", () => {
  beforeEach(() => {
    mocks.storeValue = null;
    mocks.isPending = false;
    mocks.setStored.mockClear();
  });

  it("restores the persisted GPU selection on mount", () => {
    mocks.storeValue = "nvapi:12345";

    const { result } = renderHook(() => useHarness(), { wrapper });

    expect(result.current.selected).toBe("nvapi:12345");
  });

  it("does not write the pre-hydration value back over the restored one", () => {
    // The write-back effect runs in the same commit as hydration, where the
    // atom still holds null. Persisting there would erase the preference the
    // hydration effect had just restored.
    mocks.storeValue = "nvapi:12345";

    renderHook(() => useHarness(), { wrapper });

    expect(mocks.setStored).not.toHaveBeenCalled();
  });

  it("persists an explicit change", () => {
    mocks.storeValue = "nvapi:12345";

    const { result } = renderHook(() => useHarness(), { wrapper });
    act(() => result.current.setSelected("pci:0:2:0"));

    expect(mocks.setStored).toHaveBeenCalledWith("pci:0:2:0");
  });

  it("restores a selection the current session cannot resolve, rather than discarding it", () => {
    // An adapter that is absent this launch — an unplugged eGPU. The stored
    // intent has to survive so it applies again when the adapter returns;
    // resolving it for display is `getEffectiveGpuId`'s job, not this hook's.
    mocks.storeValue = "nvapi:absent";

    const { result } = renderHook(() => useHarness(), { wrapper });

    expect(result.current.selected).toBe("nvapi:absent");
    expect(mocks.setStored).not.toHaveBeenCalled();
  });

  it("waits for the store to load before hydrating", () => {
    mocks.isPending = true;
    mocks.storeValue = "nvapi:12345";

    const { result } = renderHook(() => useHarness(), { wrapper });

    expect(result.current.selected).toBeNull();
    expect(mocks.setStored).not.toHaveBeenCalled();
  });
});
