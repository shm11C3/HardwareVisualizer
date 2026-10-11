import { useAtomValue } from "jotai";
import {
  type LiveSeriesChannel,
  liveSeriesAtom,
} from "@/features/hardware/store/liveMetrics";

/**
 * A channel's usage window, oldest first and padded with `null` to
 * `chartConfig.historyLengthSec`. A new array with every sample, because the
 * window slides; read a scalar with `useLiveScalar` when only the current value
 * is needed.
 */
export const useLiveSeries = (channel: LiveSeriesChannel): (number | null)[] =>
  useAtomValue(liveSeriesAtom(channel));
