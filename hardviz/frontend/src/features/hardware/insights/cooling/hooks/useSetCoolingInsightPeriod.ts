import { useSetAtom } from "jotai";
import { coolingInsightPeriodAtom } from "@/features/hardware/store/coolingPeriod";

/** Select a Cooling Insight period; persistence follows from the atom. */
export const useSetCoolingInsightPeriod = () =>
  useSetAtom(coolingInsightPeriodAtom);
