import { useAtom } from "jotai";
import { useCallback, useEffect, useRef, useState } from "react";
import { getProcessGpuStats } from "@/features/hardware/insights/process/funcs/getProcessStatsRecord";
import type { ProcessStatsPeriod } from "@/features/hardware/insights/utils/archivePeriod";
import { processGpuStatsAtom } from "@/features/hardware/store/processStats";

/**
 * Archived per-process GPU usage for the window `useProcessStats` reads
 * (pass its `endAt`), refreshed on the same cadence. It is a separate read so
 * a GPU failure leaves the CPU and memory table in place (DP-02).
 */
export const useProcessGpuStats = ({
  period,
  endAt,
}: {
  period: ProcessStatsPeriod;
  endAt: Date;
}) => {
  const [hasError, setHasError] = useState(false);
  const [processGpuStats, setProcessGpuStats] = useAtom(processGpuStatsAtom);

  const requestIdRef = useRef(0);

  const load = useCallback(async () => {
    // A superseded read (new period/endAt, retry, unmount) must not flip
    // the state of the read that replaced it.
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    try {
      const stats = await getProcessGpuStats(period, endAt);
      if (requestIdRef.current !== requestId) {
        return;
      }
      setProcessGpuStats(stats);
      setHasError(false);
    } catch (err) {
      console.error(err);
      if (requestIdRef.current === requestId) {
        setHasError(true);
      }
    }
  }, [period, endAt, setProcessGpuStats]);

  useEffect(() => {
    void load();

    const interval = setInterval(() => void load(), 60000); // Same cadence as useProcessStats

    return () => {
      requestIdRef.current += 1;
      clearInterval(interval);
    };
  }, [load]);

  const retry = useCallback(() => {
    void load();
  }, [load]);

  return { processGpuStats, hasError, retry };
};
