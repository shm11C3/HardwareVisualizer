import { useAtomValue } from "jotai";
import { coolingInsightPeriodHydratedAtom } from "@/features/hardware/store/coolingPeriod";
import { CoolingPeriodSelect } from "./components/CoolingPeriodSelect";
import { CovariateComparisonPanel } from "./components/CovariateComparisonPanel";
import { CoverageStrip } from "./components/CoverageStrip";
import { LoadBandComparisonPanel } from "./components/LoadBandComparisonPanel";
import { LoadTemperatureExplorerPanel } from "./components/LoadTemperatureExplorerPanel";
import { ObservationStrip } from "./components/ObservationStrip";
import { SensorStatusNote } from "./components/SensorStatusNote";
import { ThermalTimelineLane } from "./components/ThermalTimelineLane";
import { useCoolingInsightPeriodPersistence } from "./hooks/useCoolingInsightPeriodPersistence";

/**
 * The Cooling tab: zone structure, single period selector, and
 * empty/coverage states. Zone (2) is the synchronized thermal timeline
 * (#2019); zone (1) is the idle-drift observation strip and zone (5) is the
 * load-band comparison (#2020) - see each component's doc comment for its
 * own responsibilities. The load-vs-temperature Explorer (#2023) closes the
 * view as a collapsed secondary analysis below the comparison.
 *
 * The view only lays the panels out. Each panel reads what it renders: a
 * read that several panels share is an atom in `store/` (the period, the
 * baseline delta, the daily and fan rollups and the archive timeline), a
 * read one panel owns is a hook called in that panel.
 */
export const CoolingInsightView = () => {
  useCoolingInsightPeriodPersistence();
  const periodHydrated = useAtomValue(coolingInsightPeriodHydratedAtom);

  // The stored period has not landed yet; mounting the panels now would
  // flash, and fetch for, the default period.
  if (!periodHydrated) {
    return null;
  }

  return (
    <div className="space-y-4 pb-6">
      <div className="flex items-center justify-end">
        <CoolingPeriodSelect />
      </div>

      <ObservationStrip />
      <ThermalTimelineLane />
      <SensorStatusNote />
      <CoverageStrip />
      <LoadBandComparisonPanel />
      <CovariateComparisonPanel />
      <LoadTemperatureExplorerPanel />
    </div>
  );
};
