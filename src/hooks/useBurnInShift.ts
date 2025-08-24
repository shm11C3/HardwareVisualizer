import { useEffect, useRef } from "react";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import type { BurnInShiftPreset } from "@/rspc/bindings";

export interface BurnInOptions {
  /** Override interval (ms) for jump */
  intervalMs?: number;
  /** Override amplitude (±px) */
  amplitudePx?: number;
  /** Idle threshold in ms (default 10s) */
  idleThresholdMs?: number;
  /** Drift cycle duration (sec) */
  driftDurationSec?: number;
}

const PRESETS: Record<
  BurnInShiftPreset,
  { intervalMs: [number, number]; ampPx: [number, number]; driftSec: number }
> = {
  gentle: {
    intervalMs: [5 * 60_000, 10 * 60_000],
    ampPx: [1, 2],
    driftSec: 30,
  },
  balanced: { intervalMs: [60_000, 120_000], ampPx: [4, 8], driftSec: 24 },
  aggressive: { intervalMs: [30_000, 60_000], ampPx: [16, 48], driftSec: 40 },
};

const randInt = (min: number, max: number) =>
  Math.floor(min + Math.random() * (max - min + 1));

export const useBurnInShift = (
  ref: React.RefObject<HTMLElement | null>,
  opts?: BurnInOptions,
) => {
  const {
    intervalMs,
    amplitudePx,
    idleThresholdMs = 10_000,
    driftDurationSec,
  } = opts || {};

  const { settings } = useSettingsAtom();

  const idleTimer = useRef<number | null>(null);
  const isIdle = useRef(!settings.burnInShiftIdleOnly);
  const intervalId = useRef<number | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el || !settings.burnInShift) return;

    // Resolve preset values
    const p = PRESETS[settings.burnInShiftPreset];
    const amp = amplitudePx ?? randInt(p.ampPx[0], p.ampPx[1]);
    const interval = intervalMs ?? randInt(p.intervalMs[0], p.intervalMs[1]);
    console.log(interval);
    const driftDuration = driftDurationSec ?? p.driftSec;

    // CSS vars for drift
    el.style.setProperty("--drift-duration", `${driftDuration}s`);
    el.style.setProperty("--shift-x-start", `${-amp}px`);
    el.style.setProperty("--shift-x-mid", `${amp}px`);
    el.style.setProperty("--shift-x-end", `${-amp}px`);
    el.style.setProperty("--shift-y-start", `${-amp}px`);
    el.style.setProperty("--shift-y-mid", `${amp}px`);
    el.style.setProperty("--shift-y-end", `${-amp}px`);

    // Idle detection
    const armIdle = () => {
      if (!settings.burnInShiftIdleOnly) return;
      isIdle.current = false;
      if (idleTimer.current) clearTimeout(idleTimer.current);
      idleTimer.current = window.setTimeout(() => {
        isIdle.current = true;
      }, idleThresholdMs);
    };
    const onInput = () => armIdle();

    if (settings.burnInShiftIdleOnly) {
      window.addEventListener("mousemove", onInput);
      window.addEventListener("keydown", onInput);
      window.addEventListener("wheel", onInput, { passive: true });
      armIdle();
    } else {
      isIdle.current = true;
    }

    // Mode: jump
    if (settings.burnInShiftMode === "jump") {
      const jump = () => {
        if (!isIdle.current) return;
        const x = randInt(-amp, amp);
        const y = randInt(-amp, amp);
        el.style.setProperty("--shift-x", `${x}px`);
        el.style.setProperty("--shift-y", `${y}px`);
      };
      intervalId.current = window.setInterval(jump, interval);
    }

    return () => {
      if (intervalId.current) {
        clearInterval(intervalId.current);
        intervalId.current = null;
      }
      if (settings.burnInShiftIdleOnly) {
        window.removeEventListener("mousemove", onInput);
        window.removeEventListener("keydown", onInput);
        window.removeEventListener("wheel", onInput);
      }
      if (idleTimer.current) {
        clearTimeout(idleTimer.current);
        idleTimer.current = null;
      }
    };
  }, [
    ref,
    settings.burnInShift,
    settings.burnInShiftMode,
    settings.burnInShiftPreset,
    settings.burnInShiftIdleOnly,
    intervalMs,
    amplitudePx,
    idleThresholdMs,
    driftDurationSec,
  ]);
};
