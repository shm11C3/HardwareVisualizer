import { expect, test } from "@playwright/test";
import { gotoApp } from "../helpers";
import {
  analyzeMemoryGrowth,
  collectMemorySamples,
  createMemorySession,
  type MemorySample,
  type MemoryThresholds,
  writeMemoryReport,
} from "./renderMemory";

const PERF_BOOTSTRAP_TIMEOUT = 60_000;
const CONFIG = {
  warmupMs: Number(process.env.RENDER_MEMORY_WARMUP_MS ?? 10_000),
  durationMs: Number(process.env.RENDER_MEMORY_DURATION_MS ?? 45_000),
  sampleIntervalMs: Number(
    process.env.RENDER_MEMORY_SAMPLE_INTERVAL_MS ?? 5_000,
  ),
  tailWindowMs: Number(process.env.RENDER_MEMORY_TAIL_WINDOW_MS ?? 20_000),
  streamIntervalMs: Number(
    process.env.RENDER_MEMORY_STREAM_INTERVAL_MS ?? 1_000,
  ),
};
const THRESHOLDS: MemoryThresholds = {
  jsHeapGrowthBytes: 20 * 1024 * 1024,
  jsHeapSlopeBytesPerMinute: 8 * 1024 * 1024,
  domNodeGrowth: 300,
  listenerGrowth: 50,
};

test.describe("frontend memory growth", () => {
  test("dashboard mock IPC stream memory stays observable", async ({
    page,
    context,
  }) => {
    await gotoApp(page, { timeout: PERF_BOOTSTRAP_TIMEOUT });

    const cdp = await context.newCDPSession(page);
    await createMemorySession(cdp);

    await page.evaluate(async (streamIntervalMs) => {
      if (!window.__E2E__) {
        throw new Error(
          "window.__E2E__ is not installed (VITE_E2E_MOCK unset?)",
        );
      }
      await window.__E2E__.startHardwareUpdateStream({
        intervalMs: streamIntervalMs,
      });
    }, CONFIG.streamIntervalMs);

    let emittedCount = 0;
    let samples: MemorySample[] = [];
    try {
      samples = await collectMemorySamples(cdp, CONFIG);
    } finally {
      const result = await page.evaluate(async () =>
        window.__E2E__?.stopHardwareUpdateStream(),
      );
      emittedCount = result?.emittedCount ?? 0;
      await cdp.detach();
    }

    const analysis = analyzeMemoryGrowth(
      samples,
      THRESHOLDS,
      CONFIG.tailWindowMs,
    );
    await writeMemoryReport({
      generatedAt: new Date().toISOString(),
      scenario: "dashboard-mock-ipc-stream",
      config: {
        ...CONFIG,
        emittedCount,
      },
      thresholds: THRESHOLDS,
      analysis,
      samples,
    });

    if (analysis.status === "warn") {
      console.warn(
        `Frontend memory growth warning: ${analysis.reasons.join("; ")}`,
      );
    }

    expect(samples.length).toBeGreaterThanOrEqual(3);
    expect(analysis.baselineElapsedMs).toBeGreaterThanOrEqual(CONFIG.warmupMs);
  });
});
