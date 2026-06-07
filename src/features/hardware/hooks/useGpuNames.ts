import { useEffect, useState } from "react";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const useGpuNames = () => {
  const [gpuNames, setGpuNames] = useState<string[]>([]);
  const { error } = useTauriDialog();

  useEffect(() => {
    const fetchGpuNames = async () => {
      try {
        const result = await commands.getGpuArchiveNames();
        if (isError(result)) {
          await error(`Failed to fetch archived GPU names: ${result.error}`);
          setGpuNames([]);
          return;
        }
        setGpuNames(result.data);
      } catch (err) {
        await error(`Failed to fetch archived GPU names: ${String(err)}`);
        setGpuNames([]);
      }
    };

    void fetchGpuNames();
  }, [error]);

  return gpuNames;
};
