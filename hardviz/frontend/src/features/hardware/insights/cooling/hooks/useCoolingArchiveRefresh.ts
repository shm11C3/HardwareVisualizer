import { useAtomValue, useSetAtom } from "jotai";
import { useEffect } from "react";
import { chartConfig } from "@/consts/chart";
import { coolingArchiveMinutesAtom } from "@/features/hardware/store/coolingPeriod";
import { coolingArchiveAtom } from "@/features/hardware/store/coolingReads";

/**
 * Re-reads the archive timeline every archive write interval while the
 * calling panel is mounted and the period routes to the archive (24h, 7d,
 * 30d). The daily-rollup periods have nothing to refresh.
 *
 * The timer lives here rather than in the atom so that nothing fetches while
 * the panel is unmounted. It does not pause for a hidden document: the
 * archive read this replaces never did either. The interval restarts when
 * the period changes, because that change already fetches.
 */
export const useCoolingArchiveRefresh = () => {
  const minutes = useAtomValue(coolingArchiveMinutesAtom);
  const refresh = useSetAtom(coolingArchiveAtom);

  useEffect(() => {
    if (minutes == null) {
      return;
    }
    const intervalId = window.setInterval(
      () => refresh("refresh"),
      chartConfig.archiveUpdateIntervalMilSec,
    );

    return () => {
      clearInterval(intervalId);
    };
  }, [minutes, refresh]);
};
