import { useEffect, useState } from "react";
import { sqlitePromise } from "@/lib/sqlite";

export const useGpuNames = () => {
  const [gpuNames, setGpuNames] = useState<string[]>([]);

  useEffect(() => {
    const fetchGpuNames = async () => {
      const db = await sqlitePromise;
      const result = await db.load<{ gpu_name: string }>(
        "SELECT DISTINCT gpu_name FROM GPU_DATA_ARCHIVE WHERE gpu_name IS NOT NULL AND gpu_name != 'Unknown'",
      );
      setGpuNames(result.map((row) => row.gpu_name));
    };

    fetchGpuNames();
  }, []);

  return gpuNames;
};
