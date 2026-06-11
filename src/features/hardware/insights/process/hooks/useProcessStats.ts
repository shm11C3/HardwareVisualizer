import { useEffect, useState } from "react";
import {
  type archivePeriods,
  chartConfig,
} from "@/features/hardware/consts/chart";
import { useTauriDialog } from "@/hooks/useTauriDialog";
import { getProcessStats } from "../funcs/getProcessStatsRecord";
import { useProcessStatsAtom } from "./useProcessStatsAtom";

export const useProcessStats = ({
  period,
  offset,
}: {
  period: (typeof archivePeriods)[number];
  offset: number;
}) => {
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

  useEffect(() => {
    const endAt = new Date(Date.now() - offset * step);

    const fetchStats = async () => {
      try {
        setLoading(true);
        const stats = await getProcessStats(period, endAt);
        setProcessStatsAtom(stats);
      } catch (err) {
        console.error(err);
        error(String(err));
      } finally {
        setLoading(false);
      }
    };

    fetchStats();

    const interval = setInterval(fetchStats, 60000); // Update every 1 minute

    return () => clearInterval(interval);
  }, [period, offset, step, setProcessStatsAtom, error]);

  return { processStats, loading };
};
