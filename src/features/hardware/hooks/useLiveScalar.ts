import { useAtomValue } from "jotai";
import {
  type LiveScalarChannel,
  liveScalarAtom,
} from "@/features/hardware/store/liveMetrics";

/**
 * A channel's current value, or `null` before its first sample. A primitive,
 * so the component re-renders only when the value itself changes.
 */
export const useLiveScalar = (channel: LiveScalarChannel): number | null =>
  useAtomValue(liveScalarAtom(channel));
