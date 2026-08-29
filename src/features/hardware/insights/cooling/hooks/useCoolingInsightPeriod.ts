import { useTauriStore } from "@/hooks/useTauriStore";
import type { CoolingInsightPeriod } from "../types";

/**
 * UI-local key for the Cooling tab's single period selector. Deliberately
 * separate from the per-chart `period*` Tauri Store keys the CPU/Memory tab
 * still uses (`periodAvgCpuTemperature`, etc.) - those stay owned by that
 * tab, and sharing one key across two tabs was the pre-#2018 bug this
 * dedicated key avoids reintroducing.
 */
export const COOLING_INSIGHT_PERIOD_STORE_KEY = "coolingInsightPeriod";

export const useCoolingInsightPeriod = () =>
  useTauriStore<CoolingInsightPeriod>(COOLING_INSIGHT_PERIOD_STORE_KEY, "24h");
