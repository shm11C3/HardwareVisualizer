import { useAtom } from "jotai";
import { useCallback, useEffect, useRef } from "react";
import { getProcessGpuStats } from "@/features/hardware/insights/process/funcs/getProcessStatsRecord";
import type { ProcessStatsPeriod } from "@/features/hardware/insights/utils/archivePeriod";
import { processGpuReadAtom } from "@/features/hardware/store/processStats";

/**
 * Archived per-process GPU usage for the window `useProcessStats` reads
 * (pass its `endAt`), refreshed on the same cadence. It is a separate read so
 * a GPU failure leaves the CPU and memory table in place (DP-02). Each result
 * and failure carries the window it answers, so the caller can tell a read
 * for the shown stats from one left over from another period.
 */
export const useProcessGpuStats = ({
  period,
  endAt,
}: {
  period: ProcessStatsPeriod;
  endAt: Date;
}) => {
  const [read, setRead] = useAtom(processGpuReadAtom);

  const requestIdRef = useRef(0);

  const load = useCallback(async () => {
    // A superseded read (new period/endAt, retry, unmount) must not flip
    // the state of the read that replaced it.
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;
    const queryWindow = { period, endAt };

    try {
      const stats = await getProcessGpuStats(period, endAt);
      if (requestIdRef.current !== requestId) {
        return;
      }
      setRead({ window: queryWindow, status: "ready", stats });
    } catch (err) {
      console.error(err);
      if (requestIdRef.current === requestId) {
        setRead({ window: queryWindow, status: "failed" });
      }
    }
  }, [period, endAt, setRead]);

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

  return { read, retry };
};
