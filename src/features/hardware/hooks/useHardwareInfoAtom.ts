import { atom, useAtom, useAtomValue, useSetAtom } from "jotai";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { commands, type NetworkInfo, type SysInfo } from "@/rspc/bindings";
import { isError } from "@/types/result";

const hardInfoAtom = atom<SysInfo>({
  cpu: null,
  memory: null,
  gpus: null,
  storage: [],
  motherboard: null,
});

const networkInfoAtom = atom<NetworkInfo[]>([]);

const fetchHardwareInfo = async (): Promise<SysInfo> => {
  const result = await commands.getHardwareInfo();
  if (!result || isError(result)) {
    const errorMsg =
      result && isError(result)
        ? result.error
        : "Failed to fetch hardware info";
    console.error("Failed to fetch hardware info:", result);
    throw new Error(
      typeof errorMsg === "string" ? errorMsg : "Failed to fetch hardware info",
    );
  }
  return result.data;
};

const fetchNetworkInfo = async (): Promise<NetworkInfo[]> => {
  const result = await commands.getNetworkInfo();
  if (!result || isError(result)) {
    const errorMsg =
      result && isError(result) ? result.error : "Failed to fetch network info";
    console.error("Failed to fetch network info:", result);
    throw new Error(
      typeof errorMsg === "string" ? errorMsg : "Failed to fetch network info",
    );
  }
  return result.data;
};

// ---------------------------------------------------------------------------
// Lazy-initialized writable atoms.
// The read function kicks off the fetch on first access, so there is no
// module-level side-effect (important for tests where Tauri is not available).
// Subsequent writes (e.g. from useRefreshHardwareInfo) replace the Promise.
// ---------------------------------------------------------------------------

let hardwareInfoInitPromise: Promise<SysInfo> | null = null;

export const hardwareInfoPromiseAtom = atom(
  (get) => {
    // The stored value is only meaningful after the first write.
    // On the very first read we bootstrap the fetch ourselves.
    const stored = get(hardwareInfoPromiseWritableAtom);
    if (stored) return stored;
    // Lazily start the first fetch (cached so concurrent reads share it)
    if (!hardwareInfoInitPromise) {
      hardwareInfoInitPromise = fetchHardwareInfo();
    }
    return hardwareInfoInitPromise;
  },
  (_get, set, promise: Promise<SysInfo>) => {
    set(hardwareInfoPromiseWritableAtom, promise);
  },
);

/** Internal backing atom – holds the latest Promise after the first write. */
const hardwareInfoPromiseWritableAtom = atom<Promise<SysInfo> | null>(null);

let networkInfoInitPromise: Promise<NetworkInfo[]> | null = null;

export const networkInfoPromiseAtom = atom(
  (get) => {
    const stored = get(networkInfoPromiseWritableAtom);
    if (stored) return stored;
    if (!networkInfoInitPromise) {
      networkInfoInitPromise = fetchNetworkInfo();
    }
    return networkInfoInitPromise;
  },
  (_get, set, promise: Promise<NetworkInfo[]>) => {
    set(networkInfoPromiseWritableAtom, promise);
  },
);

const networkInfoPromiseWritableAtom = atom<Promise<NetworkInfo[]> | null>(
  null,
);

export const useRefreshHardwareInfo = () => {
  const setPromise = useSetAtom(hardwareInfoPromiseAtom);
  return () => setPromise(fetchHardwareInfo());
};

export const useHardwareInfoAtom = () => {
  const [hardwareInfo, setHardInfo] = useAtom(hardInfoAtom);
  const [networkInfo, setNetworkInfo] = useAtom(networkInfoAtom);
  const { error } = useTauriDialog();

  const init = async () => {
    const fetchedHardwareInfo = await commands.getHardwareInfo();
    if (isError(fetchedHardwareInfo)) {
      error(fetchedHardwareInfo.error);
      console.error("Failed to fetch hardware info:", fetchedHardwareInfo);
      return;
    }

    setHardInfo(fetchedHardwareInfo.data);
  };

  const initNetwork = async () => {
    const fetchedNetworkInfo = await commands.getNetworkInfo();
    if (isError(fetchedNetworkInfo)) {
      error(fetchedNetworkInfo.error);
      console.error("Failed to fetch network info:", fetchedNetworkInfo);
      return;
    }

    setNetworkInfo(fetchedNetworkInfo.data);
  };

  const fetchMemoryInfoDetail = async () => {
    const backup = hardwareInfo.memory;
    setHardInfo({ ...hardwareInfo, memory: null });

    const result = await commands.getMemoryInfoDetail();

    if (isError(result)) {
      error(result.error);
      console.error("Failed to fetch memory info detail:", result);
      setHardInfo({ ...hardwareInfo, memory: backup });
      return;
    }

    setHardInfo({ ...hardwareInfo, memory: result.data });
  };

  return {
    hardwareInfo,
    networkInfo,
    init,
    initNetwork,
    fetchMemoryInfoDetail,
  };
};

export const useHardwareInfoSuspense = () => {
  return useAtomValue(hardwareInfoPromiseAtom);
};

export const useNetworkInfoSuspense = () => {
  return useAtomValue(networkInfoPromiseAtom);
};

export const useFetchMemoryInfoDetail = () => {
  const setPromise = useSetAtom(hardwareInfoPromiseAtom);

  return () => {
    const promise = (async (): Promise<SysInfo> => {
      // Fetch current hardware info first to preserve other fields
      const currentResult = await commands.getHardwareInfo();
      if (isError(currentResult)) {
        throw new Error(currentResult.error);
      }

      const detailResult = await commands.getMemoryInfoDetail();
      if (isError(detailResult)) {
        console.error("Failed to fetch memory info detail:", detailResult);
        throw new Error(detailResult.error);
      }

      return { ...currentResult.data, memory: detailResult.data };
    })();

    setPromise(promise);
    return promise;
  };
};
