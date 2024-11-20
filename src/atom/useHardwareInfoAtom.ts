import { type SysInfo, commands } from "@/rspc/bindings";
import { atom, useAtom } from "jotai";

const hardInfoAtom = atom<SysInfo>({
  cpu: null,
  memory: null,
  gpus: null,
});

export const useHardwareInfoAtom = () => {
  const [hardwareInfo, setHardInfo] = useAtom(hardInfoAtom);

  const init = async () => {
    const fetchedHardwareInfo = await commands.getHardwareInfo();

    if (fetchedHardwareInfo.status === "error") {
      console.error("Failed to fetch hardware info:", fetchedHardwareInfo);
      return;
    }

    setHardInfo(fetchedHardwareInfo.data);
  };

  return { hardwareInfo, init };
};
