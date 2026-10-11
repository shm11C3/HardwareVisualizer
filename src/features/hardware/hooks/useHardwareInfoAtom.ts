import { useAtom } from "jotai";
import {
  hardInfoAtom,
  hardwareInfoLoadFailedAtom,
  networkInfoAtom,
  networkInfoLoadFailedAtom,
} from "@/features/hardware/store/hardwareInfo";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const useHardwareInfoAtom = () => {
  const [hardwareInfo, setHardInfo] = useAtom(hardInfoAtom);
  const [networkInfo, setNetworkInfo] = useAtom(networkInfoAtom);
  const [inventoryLoadFailed, setInventoryLoadFailed] = useAtom(
    hardwareInfoLoadFailedAtom,
  );
  const [networkLoadFailed, setNetworkLoadFailed] = useAtom(
    networkInfoLoadFailedAtom,
  );

  /** Also the retry for the specification sheet: it clears the failure on success. */
  const init = async () => {
    const fetchedHardwareInfo = await commands.getHardwareInfo();
    if (isError(fetchedHardwareInfo)) {
      console.error("Failed to fetch hardware info:", fetchedHardwareInfo);
      setInventoryLoadFailed(true);
      return;
    }

    setHardInfo(fetchedHardwareInfo.data);
    setInventoryLoadFailed(false);
  };

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
    const backup = hardwareInfo.memory;
    setHardInfo({ ...hardwareInfo, memory: null });

    const result = await commands.getMemoryInfoDetail();

    if (isError(result)) {
      console.error("Failed to fetch memory info detail:", result);
      setHardInfo({ ...hardwareInfo, memory: backup });
      return false;
    }

    setHardInfo({ ...hardwareInfo, memory: result.data });
    return true;
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
