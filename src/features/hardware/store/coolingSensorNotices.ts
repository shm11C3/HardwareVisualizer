import { atom, type Getter } from "jotai";
import {
  namedAmbientSources,
  resolveRoutedAmbientCapability,
} from "@/features/hardware/insights/cooling/utils/ambientTimeline";
import { resolveRoutedFanCapability } from "@/features/hardware/insights/cooling/utils/fanTimeline";
import { resolveSensorNotice } from "@/features/hardware/insights/cooling/utils/sensorNotice";
import {
  EMPTY_ARCHIVE_TIMELINE_SERIES,
  resolveRoutedPowerCapability,
} from "@/features/hardware/insights/cooling/utils/thermalTimeline";
import { coolingPeriodRouteAtom } from "@/features/hardware/store/coolingPeriod";
import {
  coolingArchiveAtom,
  coolingDailyTrendAtom,
  coolingFanTrendAtom,
} from "@/features/hardware/store/coolingReads";
import { cpuPowerSupportAtom } from "@/features/hardware/store/power";
import { motherboardFanSupportAtom } from "@/features/hardware/store/sensors";

/**
 * What the Cooling panels may say about a missing power or fan lane or an
 * ambient source. Derived from the shared reads, so the sensor-status note,
 * the timeline's own lane gates and the data-state panel cannot disagree.
 */

// The archive in the shape the capability resolvers take: a read that has
// not landed, or that failed, says nothing about the window.
const archiveInput = (get: Getter) => {
  const archive = get(coolingArchiveAtom);
  if (archive.kind === "ready") {
    return { ...archive.data, hasLoaded: true, hasError: false };
  }
  return {
    series: EMPTY_ARCHIVE_TIMELINE_SERIES,
    fanSeries: [],
    fanHasError: false,
    ambientSeries: null,
    ambientHasError: false,
    hasLoaded: archive.kind === "failed",
    hasError: archive.kind === "failed",
  };
};

/** Historical absence says only that this route has no values; Core's
 * hardware-support fact decides whether to explain why the lane is gone. */
export const coolingPowerNoticeAtom = atom((get) => {
  const archive = archiveInput(get);
  const daily = get(coolingDailyTrendAtom);
  return resolveSensorNotice(
    resolveRoutedPowerCapability(get(coolingPeriodRouteAtom), archive, {
      points: daily.kind === "ready" ? daily.data : null,
      hasError: daily.kind === "failed",
    }),
    get(cpuPowerSupportAtom),
  );
});

/** Power and fan are resolved separately: a machine can support either one. */
export const coolingFanNoticeAtom = atom((get) => {
  const archive = archiveInput(get);
  const daily = get(coolingDailyTrendAtom);
  const fan = get(coolingFanTrendAtom);
  return resolveSensorNotice(
    resolveRoutedFanCapability(
      get(coolingPeriodRouteAtom),
      {
        fanSeries: archive.fanSeries,
        cpuSeries: archive.series,
        hasLoaded: archive.hasLoaded,
        hasError: archive.hasError,
        fanHasError: archive.fanHasError,
      },
      {
        fanSeries: fan.kind === "ready" ? fan.data.series : null,
        archiveHasReadings:
          fan.kind === "ready" ? fan.data.archiveHasReadings : false,
        recordedDays: daily.kind === "ready" ? daily.data.length : null,
        hasError: fan.kind === "failed" || daily.kind === "failed",
      },
    ),
    get(motherboardFanSupportAtom),
  );
});

const NO_SOURCES: readonly string[] = [];

/**
 * Sensor Source Labels the data-state panel may name: only a routed window
 * that actually carries ambient licenses naming the sensors behind it. The
 * co-variate panel does not read this - it compares its own baseline and
 * recent windows, so a routed period without ambient readings must not hide
 * it.
 */
export const coolingAmbientSourcesAtom = atom((get) => {
  const archive = archiveInput(get);
  const capability = resolveRoutedAmbientCapability(
    get(coolingPeriodRouteAtom),
    {
      ambientSeries: archive.ambientSeries,
      cpuSeries: archive.series,
      hasLoaded: archive.hasLoaded,
      hasError: archive.hasError,
      ambientHasError: archive.ambientHasError,
    },
  );
  const sources = namedAmbientSources(capability, archive.ambientSeries);
  return sources.length === 0 ? NO_SOURCES : sources;
});
