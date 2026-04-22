import { useAtom } from "jotai";
import { useEffect, useRef } from "react";
import { useHardwareInfoAtom } from "@/features/hardware/hooks/useHardwareInfoAtom";
import { selectedGpuIdAtom } from "@/features/hardware/store/chart";
import { useTauriStore } from "@/hooks/useTauriStore";

const STORE_KEY = "selectedGpuId";

/**
 * Persist the currently selected GPU id via Tauri Store so it survives
 * app restarts. Stored values referencing GPUs that no longer exist are
 * ignored so the event listener's auto-selection of the first GPU wins.
 */
export const useSelectedGpuPersistence = () => {
  const [storedId, setStoredId, isPending] = useTauriStore<string | null>(
    STORE_KEY,
    null,
  );
  const [selectedGpuId, setSelectedGpuId] = useAtom(selectedGpuIdAtom);
  const { hardwareInfo } = useHardwareInfoAtom();
  const hydratedRef = useRef(false);

  useEffect(() => {
    if (isPending || hydratedRef.current) return;
    const gpus = hardwareInfo.gpus ?? [];
    if (gpus.length === 0) return;
    if (storedId && gpus.some((g) => g.id === storedId)) {
      setSelectedGpuId(storedId);
    }
    hydratedRef.current = true;
  }, [isPending, storedId, hardwareInfo.gpus, setSelectedGpuId]);

  useEffect(() => {
    if (!hydratedRef.current) return;
    if (selectedGpuId === storedId) return;
    setStoredId(selectedGpuId);
  }, [selectedGpuId, storedId, setStoredId]);
};
