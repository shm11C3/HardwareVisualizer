import { useEffect, useRef } from "react";
import { useSettingsAtom } from "@/features/settings/hooks/useSettingsAtom";
import { randInt } from "@/lib/math";
import type { BurnInShiftOptions, BurnInShiftPreset } from "@/rspc/bindings";

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

export const useBurnInShift = (
  ref: React.RefObject<HTMLElement | null>,
  enabled: boolean,
  opts?: BurnInShiftOptions,
) => {
  const {
    intervalMs,
    amplitudePx,
    idleThresholdMs = 10_000,
    driftDurationSec,
    panelScale = 100,
    panelAspect = "auto",
    roamAreaPercent = 100,
    keepWithinBounds = true,
  } = opts || {};

  const { settings } = useSettingsAtom();

  const idleTimer = useRef<number | null>(null);
  const isIdle = useRef(!settings.burnInShiftIdleOnly);
  const intervalId = useRef<number | null>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el || !settings.burnInShift || !enabled) return;

    // Apply panel scale
    const scale = (panelScale ?? 100) / 100;
    el.style.setProperty("--panel-scale", scale.toString());
    el.style.transform = `scale(${scale})`;
    el.style.transformOrigin = "top left";

    // Apply panel aspect ratio
    if (panelAspect === "compact") {
      el.style.maxWidth = "1200px";
      el.style.aspectRatio = "16/9";
    } else if (panelAspect === "tall") {
      el.style.maxWidth = "800px";
      el.style.aspectRatio = "9/16";
    } else {
      el.style.maxWidth = "";
      el.style.aspectRatio = "";
    }

    // Calculate roam boundaries
    const screenWidth = window.innerWidth;
    const screenHeight = window.innerHeight;
    const roamFactor = (roamAreaPercent ?? 100) / 100;

    // Calculate effective panel dimensions with scale
    const panelRect = el.getBoundingClientRect();
    const scaledWidth = panelRect.width * scale;
    const scaledHeight = panelRect.height * scale;

    // Calculate maximum shift to keep within bounds
    const maxShiftX = keepWithinBounds
      ? Math.max(0, (screenWidth * roamFactor - scaledWidth) / 2)
      : (screenWidth * roamFactor) / 2;
    const maxShiftY = keepWithinBounds
      ? Math.max(0, (screenHeight * roamFactor - scaledHeight) / 2)
      : (screenHeight * roamFactor) / 2;

    // Resolve preset values
    const p = PRESETS[settings.burnInShiftPreset];
    const amp = amplitudePx ?? [
      Math.min(randInt(p.ampPx[0], p.ampPx[1]), maxShiftX),
      Math.min(randInt(p.ampPx[0], p.ampPx[1]), maxShiftY),
    ];
    const interval = intervalMs ?? randInt(p.intervalMs[0], p.intervalMs[1]);
    const driftDuration = driftDurationSec ?? p.driftSec;

    // CSS vars for drift
    el.style.setProperty("--drift-duration", `${driftDuration}s`);
    el.style.setProperty(
      "--shift-x-start",
      `${-Math.min(amp[0], maxShiftX)}px`,
    );
    el.style.setProperty("--shift-x-mid", `${Math.min(amp[0], maxShiftX)}px`);
    el.style.setProperty("--shift-x-end", `${-Math.min(amp[0], maxShiftX)}px`);
    el.style.setProperty(
      "--shift-y-start",
      `${-Math.min(amp[1], maxShiftY)}px`,
    );
    el.style.setProperty("--shift-y-mid", `${Math.min(amp[1], maxShiftY)}px`);
    el.style.setProperty("--shift-y-end", `${-Math.min(amp[1], maxShiftY)}px`);

    // Idle detection
    const armIdle = () => {
      if (!settings.burnInShiftIdleOnly) return;
      isIdle.current = false;
      if (idleTimer.current) clearTimeout(idleTimer.current);
      idleTimer.current = window.setTimeout(() => {
        isIdle.current = true;
      }, idleThresholdMs ?? undefined);
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
        let x = randInt(-amp[0], amp[0]);
        let y = randInt(-amp[1], amp[1]);

        // Clamp to bounds if enabled
        if (keepWithinBounds) {
          x = Math.max(-maxShiftX, Math.min(maxShiftX, x));
          y = Math.max(-maxShiftY, Math.min(maxShiftY, y));
        }

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
      // Reset styles
      el.style.transform = "";
      el.style.transformOrigin = "";
      el.style.maxWidth = "";
      el.style.aspectRatio = "";
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
    panelScale,
    panelAspect,
    roamAreaPercent,
    keepWithinBounds,
    enabled,
  ]);
};
