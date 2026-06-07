import { useEffect, useState } from "react";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

export const useGpuNames = () => {
  const [gpuNames, setGpuNames] = useState<string[]>([]);

  useEffect(() => {
    const fetchGpuNames = async () => {
      const result = await commands.getGpuArchiveNames();
      if (isError(result)) {
        console.error("Failed to fetch archived GPU names:", result.error);
        setGpuNames([]);
        return;
      }
      setGpuNames(result.data);
    };

    fetchGpuNames();
  }, []);

  return gpuNames;
};
