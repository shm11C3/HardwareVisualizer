import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { chartConfig } from "@/consts/chart";
import type { archivePeriods } from "@/features/hardware/consts/chart";
import { getProcessStats } from "@/features/hardware/insights/process/funcs/getProcessStatsRecord";
import type { ProcessStat } from "@/features/hardware/insights/types/processStats";
import { useProcessStatsAtom } from "./useProcessStatsAtom";

export const useProcessStats = ({
  period,
  offset,
}: {
  period: (typeof archivePeriods)[number];
  offset: number;
}) => {
  const [loading, setLoading] = useState(true);
  const [hasError, setHasError] = useState(false);
  const { processStats, setProcessStatsAtom } = useProcessStatsAtom();

  const step =
    {
      10: 1,
      30: 1,
      60: 1,
      180: 1,
      720: 10,
      1440: 30,
      10080: 60,
      20160: 180,
      43200: 720,
    }[period] * chartConfig.archiveUpdateIntervalMilSec;

  const endAt = useMemo(() => {
    return new Date(Date.now() - offset * step);
  }, [offset, step]);

  const getData = useCallback(
    async (): Promise<ProcessStat[]> => getProcessStats(period, endAt),
    [period, endAt],
  );

  const requestIdRef = useRef(0);

  const load = useCallback(async () => {
    // A superseded read (new period/offset, retry, unmount) must not flip
    // the state of the read that replaced it.
    const requestId = requestIdRef.current + 1;
    requestIdRef.current = requestId;

    try {
      setLoading(true);
      const stats = await getData();
      if (requestIdRef.current !== requestId) {
        return;
      }
      setProcessStatsAtom(stats);
      setHasError(false);
    } catch (err) {
      console.error(err);
      if (requestIdRef.current === requestId) {
        setHasError(true);
      }
    } finally {
      if (requestIdRef.current === requestId) {
        setLoading(false);
      }
    }
  }, [setProcessStatsAtom, getData]);

  useEffect(() => {
    void load();

    const interval = setInterval(() => void load(), 60000); // Update every 1 minute

    return () => {
      requestIdRef.current += 1;
      clearInterval(interval);
    };
  }, [load]);

  const retry = useCallback(() => {
    void load();
  }, [load]);

  return { processStats, loading, hasError, retry };
};
