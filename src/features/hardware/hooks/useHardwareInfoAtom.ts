import { useAtom, useStore } from "jotai";
import { useCallback } from "react";
import {
  hardInfoAtom,
  hardwareInfoLoadFailedAtom,
  memoryDetailLoadingAtom,
  memoryDimmTemperatureAtom,
  networkInfoAtom,
  networkInfoLoadFailedAtom,
} from "@/features/hardware/store/hardwareInfo";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const useHardwareInfoAtom = () => {
  const store = useStore();
  const [hardwareInfo, setHardInfo] = useAtom(hardInfoAtom);
  const [networkInfo, setNetworkInfo] = useAtom(networkInfoAtom);
  const [inventoryLoadFailed, setInventoryLoadFailed] = useAtom(
    hardwareInfoLoadFailedAtom,
  );
  const [networkLoadFailed, setNetworkLoadFailed] = useAtom(
    networkInfoLoadFailedAtom,
  );

  /** Also the retry for the specification sheet: it clears the failure on success. */
  const init = useCallback(async () => {
    const fetchedHardwareInfo = await commands.getHardwareInfo();
    if (isError(fetchedHardwareInfo)) {
      console.error("Failed to fetch hardware info:", fetchedHardwareInfo);
      setInventoryLoadFailed(true);
      return;
    }

    setHardInfo((current) => {
      const inventory = fetchedHardwareInfo.data;
      // A concurrent or partial inventory read may still have the basic view.
      // Keep the more detailed result from the explicit memory request.
      if (
        store.get(memoryDimmTemperatureAtom) !== null &&
        current.memory?.isDetailed &&
        !inventory.memory?.isDetailed
      ) {
        return { ...inventory, memory: current.memory };
      }
      return inventory;
    });
    setInventoryLoadFailed(false);
  }, [setHardInfo, setInventoryLoadFailed, store]);

  const initNetwork = async () => {
    const fetchedNetworkInfo = await commands.getNetworkInfo();
    if (isError(fetchedNetworkInfo)) {
      console.error("Failed to fetch network info:", fetchedNetworkInfo);
      setNetworkLoadFailed(true);
      return;
    }

    setNetworkInfo(fetchedNetworkInfo.data);
    setNetworkLoadFailed(false);
  };

  /**
   * User-triggered. Returns whether the detail was loaded so the caller can
   * show the failure next to the control that was used.
   */
  const fetchMemoryInfoDetail = async (): Promise<boolean> => {
    if (store.get(memoryDetailLoadingAtom)) return true;
    store.set(memoryDetailLoadingAtom, true);
    try {
      const result = await commands.getMemoryInfoDetail();
      if (isError(result)) {
        console.error("Failed to fetch memory info detail:", result);
        store.set(memoryDimmTemperatureAtom, (reading) =>
          reading?.status === "available" ? { status: "unavailable" } : reading,
        );
        return false;
      }
      setHardInfo((current) => ({ ...current, memory: result.data.memory }));
      store.set(memoryDimmTemperatureAtom, result.data.dimmTemperature);
      return true;
    } finally {
      store.set(memoryDetailLoadingAtom, false);
    }
  };

  return {
    hardwareInfo,
    networkInfo,
    inventoryLoadFailed,
    networkLoadFailed,
    init,
    initNetwork,
    fetchMemoryInfoDetail,
  };
};
