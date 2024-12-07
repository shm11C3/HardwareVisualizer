import { useTauriDialog } from "@/hooks/useTauriDialog";
import { type SysInfo, commands } from "@/rspc/bindings";
import { isError } from "@/types/result";
import { atom, useAtom } from "jotai";

const hardInfoAtom = atom<SysInfo>({
  cpu: null,
  memory: null,
  gpus: null,
  storage: [],
});

export const useHardwareInfoAtom = () => {
  const [hardwareInfo, setHardInfo] = useAtom(hardInfoAtom);
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

  return { hardwareInfo, init };
};
