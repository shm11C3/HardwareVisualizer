import type {
  CoolingBandComparison,
  CoolingBaselineDelta,
} from "@/rspc/bindings";
import { CoolingPeriodSelect } from "./components/CoolingPeriodSelect";
import { CoverageStrip } from "./components/CoverageStrip";
import { LoadBandComparisonPanel } from "./components/LoadBandComparisonPanel";
import { LoadTemperatureExplorerPanel } from "./components/LoadTemperatureExplorerPanel";
import { ObservationStrip } from "./components/ObservationStrip";
import { ThermalTimelineLane } from "./components/ThermalTimelineLane";
import { UnsupportedSensorNote } from "./components/UnsupportedSensorNote";
import { useCoolingArchiveTimeline } from "./hooks/useCoolingArchiveTimeline";
import { useCoolingBandComparison } from "./hooks/useCoolingBandComparison";
import { useCoolingBaselineDelta } from "./hooks/useCoolingBaselineDelta";
import { useCoolingDailyTrend } from "./hooks/useCoolingDailyTrend";
import { useCoolingFanTrend } from "./hooks/useCoolingFanTrend";
import { useCoolingInsightPeriod } from "./hooks/useCoolingInsightPeriod";
import type { CoolingInsightPeriod } from "./types";
import {
  namedAmbientSources,
  resolveRoutedAmbientCapability,
} from "./utils/ambientTimeline";
import { resolveCoolingPeriodRoute } from "./utils/coolingPeriodRoute";
import {
  claimsFanUnsupported,
  resolveRoutedFanCapability,
} from "./utils/fanTimeline";
import {
  claimsPowerUnsupported,
  resolveRoutedPowerCapability,
} from "./utils/thermalTimeline";

/**
 * The Cooling tab: zone structure, single period selector, and
 * empty/coverage states. Zone (2) is the synchronized thermal timeline
 * (#2019); zone (1) is the idle-drift observation strip and zone (5) is the
 * load-band comparison (#2020) - see each component's doc comment for its
 * own responsibilities. The load-vs-temperature Explorer (#2023) closes the
 * view as a collapsed secondary analysis below the comparison.
 */
export const CoolingInsightView = () => {
  const periodState = useCoolingInsightPeriod();
  const { data: baselineDelta, hasError: baselineDeltaHasError } =
    useCoolingBaselineDelta();
  const { data: bandComparison, hasError: bandComparisonHasError } =
    useCoolingBandComparison();

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
      baselineDeltaHasError={baselineDeltaHasError}
      bandComparison={bandComparison}
      bandComparisonHasError={bandComparisonHasError}
    />
  );
};

const CoolingInsightBody = ({
  period,
  onPeriodChange,
  baselineDelta,
  baselineDeltaHasError,
  bandComparison,
  bandComparisonHasError,
}: {
  period: CoolingInsightPeriod;
  onPeriodChange: (period: CoolingInsightPeriod) => Promise<void>;
  baselineDelta: CoolingBaselineDelta | null;
  baselineDeltaHasError: boolean;
  bandComparison: CoolingBandComparison | null;
  bandComparisonHasError: boolean;
}) => {
  const route = resolveCoolingPeriodRoute(period);
  const { data: dailyTrend, hasError: dailyTrendHasError } =
    useCoolingDailyTrend(route.kind === "dailyTrend" ? route.days : null);
  const { data: fanTrend, hasError: fanTrendHasError } = useCoolingFanTrend(
    route.kind === "dailyTrend" ? route.days : null,
  );
  // Owned here rather than inside the timeline: both the timeline's power
  // and fan lanes and the pending-sensors note below them read the same
  // series, and fetching them twice would double the archive round trips.
  const archive = useCoolingArchiveTimeline(
    route.kind === "archive" ? route.minutes : null,
  );

  // Only a resolved, non-empty window that recorded no watts licenses
  // telling the user power is unsupported; loading, failure, and an empty
  // window all leave the claim unmade.
  const powerUnsupported = claimsPowerUnsupported(
    resolveRoutedPowerCapability(route, archive, {
      points: dailyTrend,
      hasError: dailyTrendHasError,
    }),
  );
  // The same three-state contract for the fan, answered from its own
  // sources: a machine can have one capability without the other.
  const fanUnsupported = claimsFanUnsupported(
    resolveRoutedFanCapability(
      route,
      {
        fanSeries: archive.fanSeries,
        cpuSeries: archive.series,
        hasLoaded: archive.hasLoaded,
        hasError: archive.hasError,
        fanHasError: archive.fanHasError,
      },
      {
        fanSeries: fanTrend?.series ?? null,
        archiveHasReadings: fanTrend?.archiveHasReadings ?? false,
        recordedDays: dailyTrend?.length ?? null,
        hasError: fanTrendHasError || dailyTrendHasError,
      },
    ),
  );
  // The same three-state contract once more, for the data-state panel's
  // ambient row: only a routed window that actually carries ambient
  // licenses naming the sensors behind it.
  const ambientSources = namedAmbientSources(
    resolveRoutedAmbientCapability(route, {
      ambientSeries: archive.ambientSeries,
      cpuSeries: archive.series,
      hasLoaded: archive.hasLoaded,
      hasError: archive.hasError,
      ambientHasError: archive.ambientHasError,
    }),
    archive.ambientSeries,
  );

  return (
    <div className="space-y-4 pb-6">
      <div className="flex items-center justify-end">
        <CoolingPeriodSelect value={period} onChange={onPeriodChange} />
      </div>

      <ObservationStrip
        baselineDelta={baselineDelta}
        hasError={baselineDeltaHasError}
      />
      <ThermalTimelineLane
        route={route}
        baseline={baselineDelta?.baseline ?? null}
        dailyTrend={dailyTrend}
        fanTrend={fanTrend?.series ?? null}
        archive={archive}
      />
      <UnsupportedSensorNote
        powerUnsupported={powerUnsupported}
        fanUnsupported={fanUnsupported}
      />
      {route.kind === "dailyTrend" && (
        <CoverageStrip
          points={dailyTrend}
          days={route.days}
          hasError={dailyTrendHasError}
        />
      )}
      <LoadBandComparisonPanel
        bandComparison={bandComparison}
        hasError={bandComparisonHasError}
        powerUnsupported={powerUnsupported}
        fanUnsupported={fanUnsupported}
        ambientSources={ambientSources}
      />
      <LoadTemperatureExplorerPanel />
    </div>
  );
};
