import {
  type archivePeriods,
  chartConfig,
} from "@/features/hardware/consts/chart";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { useCallback, useEffect, useMemo, useState } from "react";
import { getProcessStats } from "../funcs/getProcessStatsRecord";
import type { ProcessStat } from "../types/processStats";
import { useProcessStatsAtom } from "./useProcessStatsAtom";

export function useProcessStats({
  period,
  offset,
}: { period: (typeof archivePeriods)[number]; offset: number }) {
  const [loading, setLoading] = useState(true);
  const { error } = useTauriDialog();
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

  useEffect(() => {
    const fetchStats = async () => {
      try {
        setLoading(true);
        const stats = await getData();
        setProcessStatsAtom(stats);
      } catch (err) {
        console.error(err);
        error(String(err));
      } finally {
        setLoading(false);
      }
    };

    fetchStats();

    const interval = setInterval(fetchStats, 60000); // 1分ごとに更新

    return () => clearInterval(interval);
  }, [setProcessStatsAtom, getData, error]);

  return { processStats, loading };
}
