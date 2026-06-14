import { useAtom } from "jotai";
import { useEffect, useRef } from "react";
import { selectedStorageDeviceIdAtom } from "@/features/hardware/store/chart";
import { useTauriStore } from "@/hooks/useTauriStore";

const STORE_KEY = "selectedStorageDeviceId";

/**
 * Persist the explicitly selected Storage Device id via Tauri Store so it
 * survives app restarts. No device-existence check is performed here: the
 * storage health records live inside the dashboard view, and
 * `buildStorageHealthSummary` already falls back to the Focus Storage Device
 * when the stored id is absent, so a stale id stays harmless and is restored
 * if the device is reconnected.
 */
export const useSelectedStorageDevicePersistence = () => {
  const [storedId, setStoredId, isPending] = useTauriStore<string | null>(
    STORE_KEY,
    null,
  );
  const [selectedStorageDeviceId, setSelectedStorageDeviceId] = useAtom(
    selectedStorageDeviceIdAtom,
  );
  const hydratedRef = useRef(false);

  useEffect(() => {
    if (isPending || hydratedRef.current) return;
    if (storedId) {
      setSelectedStorageDeviceId(storedId);
    }
    hydratedRef.current = true;
  }, [isPending, storedId, setSelectedStorageDeviceId]);

  useEffect(() => {
    if (!hydratedRef.current) return;
    if (selectedStorageDeviceId === storedId) return;
    setStoredId(selectedStorageDeviceId);
  }, [selectedStorageDeviceId, storedId, setStoredId]);
};
