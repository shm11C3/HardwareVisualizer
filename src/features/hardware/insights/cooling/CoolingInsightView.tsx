import { useAtomValue } from "jotai";
import {
  cpuPowerSupportAtom,
  motherboardFanSupportAtom,
} from "@/features/hardware/store/chart";
import type {
  CoolingBandComparison,
  CoolingBaselineDelta,
} from "@/rspc/bindings";
import { CoolingPeriodSelect } from "./components/CoolingPeriodSelect";
import { CoverageStrip } from "./components/CoverageStrip";
import { LoadBandComparisonPanel } from "./components/LoadBandComparisonPanel";
import { LoadTemperatureExplorerPanel } from "./components/LoadTemperatureExplorerPanel";
import { ObservationStrip } from "./components/ObservationStrip";
import { SensorStatusNote } from "./components/SensorStatusNote";
import { ThermalTimelineLane } from "./components/ThermalTimelineLane";
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
import { resolveRoutedFanCapability } from "./utils/fanTimeline";
import { resolveSensorNotice } from "./utils/sensorNotice";
import { resolveRoutedPowerCapability } from "./utils/thermalTimeline";

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
  const cpuPowerSupport = useAtomValue(cpuPowerSupportAtom);
  const motherboardFanSupport = useAtomValue(motherboardFanSupportAtom);
  const route = resolveCoolingPeriodRoute(period);
  const { data: dailyTrend, hasError: dailyTrendHasError } =
    useCoolingDailyTrend(route.kind === "dailyTrend" ? route.days : null);
  const { data: fanTrend, hasError: fanTrendHasError } = useCoolingFanTrend(
    route.kind === "dailyTrend" ? route.days : null,
  );
  // Owned here rather than inside the timeline: both the timeline's power
  // and fan lanes and the sensor-status note below them read the same
  // series, and fetching them twice would double the archive round trips.
  const archive = useCoolingArchiveTimeline(
    route.kind === "archive" ? route.minutes : null,
  );

  // Historical absence says only that this route has no values. Combine it
  // with Core's hardware-support fact before explaining why the lane is gone.
  const powerNotice = resolveSensorNotice(
    resolveRoutedPowerCapability(route, archive, {
      points: dailyTrend,
      hasError: dailyTrendHasError,
    }),
    cpuPowerSupport,
  );
  // Power and fan are resolved separately: a machine can support either one.
  const fanNotice = resolveSensorNotice(
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
    motherboardFanSupport,
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
      <SensorStatusNote powerNotice={powerNotice} fanNotice={fanNotice} />
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
        powerNotice={powerNotice}
        fanNotice={fanNotice}
        ambientSources={ambientSources}
      />
      <LoadTemperatureExplorerPanel />
    </div>
  );
};
