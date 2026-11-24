import { useAtom } from "jotai";
import { selectedStorageDeviceIdAtom } from "@/features/hardware/store/selection";

/**
 * The Storage Device the Storage Health Display focuses on, and the way to
 * change it. Persistence across restarts is `useSelectedStorageDevicePersistence`.
 */
export const useSelectedStorageDevice = () => {
  const [selectedStorageDeviceId, selectStorageDevice] = useAtom(
    selectedStorageDeviceIdAtom,
  );
  return { selectedStorageDeviceId, selectStorageDevice };
};
