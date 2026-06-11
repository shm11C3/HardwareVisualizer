import fs from "node:fs/promises";
import path from "node:path";
import type { Page } from "@playwright/test";

declare global {
  interface Window {
    __RENDER_PERF__?: {
      longTaskDurations: number[];
      reset: () => void;
    };
  }
}

export type RenderPerfResult = {
  screen: string;
  domElementCount: number;
  longTaskCount: number;
  maxLongTaskMs: number;
  totalLongTaskMs: number;
  readyMs?: number;
  interactionMs?: number;
};

const REPORT_DIR = path.join("test-results", "render-perf");
const REPORT_PATH = path.join(REPORT_DIR, "render-perf-results.json");

export const installRenderPerfObserver = async (page: Page) => {
  await page.addInitScript(() => {
    const state = {
      longTaskDurations: [] as number[],
      reset() {
        this.longTaskDurations = [];
      },
    };

    window.__RENDER_PERF__ = state;

    if (!("PerformanceObserver" in window)) {
      return;
    }

    try {
      const observer = new PerformanceObserver((list) => {
        for (const entry of list.getEntries()) {
          state.longTaskDurations.push(entry.duration);
        }
      });
      observer.observe({ entryTypes: ["longtask"] });
    } catch {
      // The Long Task API is Chromium-only; leave the metric empty elsewhere.
    }
  });
};

export const resetRenderPerf = async (page: Page) => {
  await page.evaluate(() => window.__RENDER_PERF__?.reset());
};

export const collectRenderPerfResult = async (
  page: Page,
  timing: Pick<RenderPerfResult, "screen" | "readyMs" | "interactionMs">,
): Promise<RenderPerfResult> => {
  const browserMetrics = await page.evaluate(() => {
    const longTaskDurations = window.__RENDER_PERF__?.longTaskDurations ?? [];
    return {
      domElementCount: document.getElementsByTagName("*").length,
      longTaskCount: longTaskDurations.length,
      maxLongTaskMs: Math.round(Math.max(0, ...longTaskDurations)),
      totalLongTaskMs: Math.round(
        longTaskDurations.reduce((sum, duration) => sum + duration, 0),
      ),
    };
  });

  return {
    ...timing,
    ...browserMetrics,
  };
};

export const writeRenderPerfReport = async (results: RenderPerfResult[]) => {
  await fs.mkdir(REPORT_DIR, { recursive: true });
  await fs.writeFile(
    REPORT_PATH,
    `${JSON.stringify(
      {
        generatedAt: new Date().toISOString(),
        results,
      },
      null,
      2,
    )}\n`,
  );
};
