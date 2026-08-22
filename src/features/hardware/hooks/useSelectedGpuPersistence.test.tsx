import { act, renderHook } from "@testing-library/react";
import { Provider, useAtom } from "jotai";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { asLiveGpuId, type LiveGpuId } from "@/features/hardware/gpuIdentity";
import {
  gpuNamesAtom,
  selectedGpuIdAtom,
} from "@/features/hardware/store/chart";
import { useSelectedGpuPersistence } from "./useSelectedGpuPersistence";

const mocks = vi.hoisted(() => ({
  storeValue: null as string | null,
  setStored: vi.fn(),
  isPending: false,
  gpus: null as { id: string; name: string }[] | null,
  init: vi.fn(),
}));

vi.mock("@/hooks/useTauriStore", () => ({
  useTauriStore: () => [mocks.storeValue, mocks.setStored, mocks.isPending],
}));

vi.mock("@/features/hardware/hooks/useHardwareInfoAtom", () => ({
  useHardwareInfoAtom: () => ({
    hardwareInfo: { gpus: mocks.gpus },
    init: mocks.init,
  }),
}));

const wrapper = ({ children }: { children: ReactNode }) => (
  <Provider>{children}</Provider>
);

/** Seeds mint live ids the way the event listener does at the boundary. */
const liveMap = (map: Record<string, string>) =>
  map as unknown as Record<LiveGpuId, string>;

const useHarness = () => {
  useSelectedGpuPersistence();
  const [selected, setSelected] = useAtom(selectedGpuIdAtom);
  const [, setNames] = useAtom(gpuNamesAtom);
  return { selected, setSelected, setNames };
};

describe("useSelectedGpuPersistence", () => {
  beforeEach(() => {
    mocks.storeValue = null;
    mocks.isPending = false;
    mocks.setStored.mockClear();
    mocks.gpus = null;
    mocks.init.mockClear();
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
    act(() => result.current.setSelected(asLiveGpuId("pci:0:2:0")));

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

  it("migrates an inventory id stored by the pre-change classic card", () => {
    // Shipped versions wrote GraphicInfo.id here. Grouped navigation never
    // mounts the classic card, so translating it anywhere but app level would
    // leave the choice inert for the life of the installation.
    mocks.storeValue = "67890";
    mocks.gpus = [
      { id: "12345", name: "NVIDIA GeForce RTX 4080" },
      { id: "67890", name: "Intel UHD Graphics 770" },
    ];

    const { result } = renderHook(() => useHarness(), { wrapper });
    expect(result.current.selected).toBe("67890");

    act(() =>
      result.current.setNames(
        liveMap({
          "nvapi:1": "NVIDIA GeForce RTX 4080",
          "pci:0:2:0": "Intel UHD Graphics 770",
        }),
      ),
    );

    expect(result.current.selected).toBe("pci:0:2:0");
    expect(mocks.setStored).toHaveBeenLastCalledWith("pci:0:2:0");
  });

  it("leaves a live id that is simply absent this session alone", () => {
    mocks.storeValue = "nvapi:absent";
    mocks.gpus = [{ id: "12345", name: "NVIDIA GeForce RTX 4080" }];

    const { result } = renderHook(() => useHarness(), { wrapper });
    act(() =>
      result.current.setNames(
        liveMap({ "nvapi:1": "NVIDIA GeForce RTX 4080" }),
      ),
    );

    expect(result.current.selected).toBe("nvapi:absent");
    expect(mocks.setStored).not.toHaveBeenCalled();
  });

  it("fetches the inventory itself when a stored id cannot be resolved", () => {
    // A restart into Monitor or Compact mounts nothing that fetches the
    // inventory, and the migration cannot read what nothing has fetched.
    mocks.storeValue = "67890";
    mocks.gpus = null;

    const { result } = renderHook(() => useHarness(), { wrapper });
    act(() =>
      result.current.setNames(
        liveMap({ "pci:0:2:0": "Intel UHD Graphics 770" }),
      ),
    );

    expect(mocks.init).toHaveBeenCalled();
  });

  it("does not fetch the inventory while the stored id resolves live", () => {
    mocks.storeValue = "pci:0:2:0";
    mocks.gpus = null;

    const { result } = renderHook(() => useHarness(), { wrapper });
    act(() =>
      result.current.setNames(
        liveMap({ "pci:0:2:0": "Intel UHD Graphics 770" }),
      ),
    );

    expect(mocks.init).not.toHaveBeenCalled();
  });

  it("waits for the store to load before hydrating", () => {
    mocks.isPending = true;
    mocks.storeValue = "nvapi:12345";

    const { result } = renderHook(() => useHarness(), { wrapper });

    expect(result.current.selected).toBeNull();
    expect(mocks.setStored).not.toHaveBeenCalled();
  });
});
