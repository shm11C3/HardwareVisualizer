import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type NetworkInfo, type SysInfo, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import { atom, useAtom } from "jotai";

const hardInfoAtom = atom<SysInfo>({
  cpu: null,
  memory: null,
  gpus: null,
  storage: [],
});

const networkInfoAtom = atom<NetworkInfo[]>([]);

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

  return { hardwareInfo, networkInfo, init, initNetwork };
};
