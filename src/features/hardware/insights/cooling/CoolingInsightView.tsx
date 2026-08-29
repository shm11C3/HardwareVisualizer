import type {
  CoolingBandComparison,
  CoolingBaselineDelta,
} from "@/rspc/bindings";
import { CoolingPeriodSelect } from "./components/CoolingPeriodSelect";
import { CoverageStrip } from "./components/CoverageStrip";
import { LegacyPowerCharts } from "./components/LegacyPowerCharts";
import { LoadBandComparisonPanel } from "./components/LoadBandComparisonPanel";
import { ObservationStrip } from "./components/ObservationStrip";
import { ThermalTimelineLane } from "./components/ThermalTimelineLane";
import { UnsupportedSensorNote } from "./components/UnsupportedSensorNote";
import { useCoolingBandComparison } from "./hooks/useCoolingBandComparison";
import { useCoolingBaselineDelta } from "./hooks/useCoolingBaselineDelta";
import { useCoolingDailyTrend } from "./hooks/useCoolingDailyTrend";
import { useCoolingInsightPeriod } from "./hooks/useCoolingInsightPeriod";
import type { CoolingInsightPeriod } from "./types";
import { resolveCoolingPeriodRoute } from "./utils/coolingPeriodRoute";

/**
 * The Cooling tab: zone structure, single period selector, and
 * empty/coverage states. Zone (2) is the synchronized thermal timeline
 * (#2019); zones (1) observation strip and (5) load-band comparison still
 * hold placeholder content pending #2020 - see each component's doc comment
 * for what is deferred.
 */
export const CoolingInsightView = () => {
  const periodState = useCoolingInsightPeriod();
  const { data: baselineDelta } = useCoolingBaselineDelta();
  const { data: bandComparison } = useCoolingBandComparison();

  // The store-backed period is not ready yet; bail out before mounting
  // `CoolingInsightBody`, which calls one more hook (the daily-trend fetch)
  // that must not appear or disappear across renders of the same instance.
  if (periodState[2]) {
    return null;
  }

  const [period, setPeriod] = periodState;

  return (
    <CoolingInsightBody
      period={period}
      onPeriodChange={setPeriod}
      baselineDelta={baselineDelta}
      bandComparison={bandComparison}
    />
  );
};

const CoolingInsightBody = ({
  period,
  onPeriodChange,
  baselineDelta,
  bandComparison,
}: {
  period: CoolingInsightPeriod;
  onPeriodChange: (period: CoolingInsightPeriod) => Promise<void>;
  baselineDelta: CoolingBaselineDelta | null;
  bandComparison: CoolingBandComparison | null;
}) => {
  const route = resolveCoolingPeriodRoute(period);
  const { data: dailyTrend } = useCoolingDailyTrend(
    route.kind === "dailyTrend" ? route.days : null,
  );

  return (
    <div className="space-y-4 pb-6">
      <div className="flex items-center justify-end">
        <CoolingPeriodSelect value={period} onChange={onPeriodChange} />
      </div>

      <ObservationStrip baselineDelta={baselineDelta} />
      <ThermalTimelineLane
        route={route}
        baseline={baselineDelta?.baseline ?? null}
        dailyTrend={dailyTrend}
      />
      <UnsupportedSensorNote />
      <LegacyPowerCharts route={route} />
      {route.kind === "dailyTrend" && (
        <CoverageStrip points={dailyTrend ?? []} days={route.days} />
      )}
      <LoadBandComparisonPanel bandComparison={bandComparison} />
    </div>
  );
};
