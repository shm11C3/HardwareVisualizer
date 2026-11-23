import { useSetAtom } from "jotai";
import { useCallback } from "react";
import {
  coolingArchiveAtom,
  coolingBaselineDeltaAtom,
  coolingDailyTrendAtom,
} from "@/features/hardware/store/coolingReads";
import type { SharedReadAtom } from "@/features/hardware/store/sharedRead";

const useRetry = <P, D>(readAtom: SharedReadAtom<P, D>) => {
  const write = useSetAtom(readAtom);
  return useCallback(() => {
    write("refresh");
  }, [write]);
};

/** Retry for the observation strip's failure state. */
export const useRetryCoolingBaselineDelta = () =>
  useRetry(coolingBaselineDeltaAtom);

/** Retry for the daily-rollup periods' failure states (timeline and strip). */
export const useRetryCoolingDailyTrend = () => useRetry(coolingDailyTrendAtom);

/** Retry for the archive periods' timeline failure state. */
export const useRetryCoolingArchive = () => useRetry(coolingArchiveAtom);
