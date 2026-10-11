import { useEffect, useState } from "react";
import { commands } from "@/rspc/bindings";
import { isError } from "@/types/result";

/**
 * Archived GPU names feed the GPU selectors, which fall back to GPU ids when
 * a name is missing, so the screen still renders without them. A failed read
 * is therefore logged only (see "Failure Reporting" in the frontend
 * architecture doc), never surfaced as a dialog.
 */
export const useGpuNames = () => {
  const [gpuNames, setGpuNames] = useState<string[]>([]);

  useEffect(() => {
    const fetchGpuNames = async () => {
      try {
        const result = await commands.getGpuArchiveNames();
        if (isError(result)) {
          console.error(`Failed to fetch archived GPU names: ${result.error}`);
          setGpuNames([]);
          return;
        }
        setGpuNames(result.data);
      } catch (err) {
        console.error(`Failed to fetch archived GPU names: ${String(err)}`);
        setGpuNames([]);
      }
    };

    void fetchGpuNames();
  }, []);

  return gpuNames;
};
